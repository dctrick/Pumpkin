use std::sync::{LazyLock, Mutex, PoisonError};

use pumpkin_util::text::{TextComponent, color::NamedColor};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use uuid::Uuid;

use crate::server::Server;
use crate::world::bossbar::{Bossbar, BossbarColor, BossbarDivisions, BossbarFlags};

pub const TPS_BAR_UUID: Uuid = Uuid::from_u128(0x7470_7362_6172_4000_8000_0000_0000_0001);

const WARN_UTILIZATION: f64 = 0.8;
const BYTES_PER_MB: u64 = 1024 * 1024;

static PROCESS: LazyLock<Option<Mutex<(System, Pid)>>> = LazyLock::new(|| {
    sysinfo::get_current_pid()
        .ok()
        .map(|pid| Mutex::new((System::new(), pid)))
});

fn used_memory_mb() -> Option<u64> {
    let mut process = PROCESS
        .as_ref()?
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let (system, pid) = &mut *process;
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[*pid]),
        false,
        ProcessRefreshKind::nothing().with_memory(),
    );
    system.process(*pid).map(|p| p.memory() / BYTES_PER_MB)
}

/// Samples MSPT, tick utilization and process memory into a boss bar.
#[must_use]
pub fn bossbar(server: &Server) -> Bossbar {
    let mspt = server.get_mspt();
    let target_mspt = server.tick_rate_manager.nanoseconds_per_tick() as f64 / 1_000_000.0;
    let utilization = mspt / target_mspt;

    let (color, named) = if utilization < WARN_UTILIZATION {
        (BossbarColor::Green, NamedColor::Green)
    } else if utilization < 1.0 {
        (BossbarColor::Yellow, NamedColor::Yellow)
    } else {
        (BossbarColor::Red, NamedColor::Red)
    };

    let ram = used_memory_mb().map_or_else(|| "unavailable".to_string(), |mb| format!("{mb} MB"));

    let title = TextComponent::text("MSPT: ")
        .add_child(TextComponent::text(format!("{mspt:.2}")).color_named(named))
        .add_child(TextComponent::text(" | Util: "))
        .add_child(TextComponent::text(format!("{:.1}%", utilization * 100.0)).color_named(named))
        .add_child(TextComponent::text(" | RAM: "))
        .add_child(TextComponent::text(ram).color_named(NamedColor::Aqua));

    Bossbar {
        uuid: TPS_BAR_UUID,
        title,
        health: utilization.clamp(0.0, 1.0) as f32,
        color,
        division: BossbarDivisions::NoDivision,
        flags: BossbarFlags::empty(),
    }
}

/// Resends the bar to every player who turned it on. Re-adding replaces the client's copy,
/// so a client that dropped the bar gets it back instead of receiving updates for an unknown id.
pub fn tick(server: &Server) {
    let mut bar = None;
    server.for_each_player(|player| {
        let enabled = player
            .tps_bar
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if *enabled {
            player.send_bossbar(bar.get_or_insert_with(|| bossbar(server)));
        }
    });
}
