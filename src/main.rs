use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::Write;
use std::io::Read;
use std::path::Path;
use std::{thread, time};

struct Event {
    subsystem: &'static str,
    name: Option<&'static str>,
    required: bool,
}

struct Category {
    name: &'static str,
    description: &'static str,
    events: &'static [Event],
}

static CATEGORIES: [Category; 10] = [
    Category {
        name: "sched",
        description: "scheduler-related events",
        events: &[
            Event {
                subsystem: "sched",
                name: Some("sched_switch"),
                required: true,
            },
            Event {
                subsystem: "sched",
                name: Some("sched_wakeup"),
                required: true,
            },
            Event {
                subsystem: "sched",
                name: Some("sched_waking"),
                required: false,
            },
            // Android / CrOS only
            Event {
                subsystem: "sched",
                name: Some("sched_blocked_reason"),
                required: false,
            },
            // Android / CrOS only
            Event {
                subsystem: "sched",
                name: Some("sched_cpu_hotplug"),
                required: false,
            },
            Event {
                subsystem: "sched",
                name: Some("sched_pi_setprio"),
                required: false,
            },
            Event {
                subsystem: "cgroup",
                name: None,
                required: false,
            },
        ],
    },
    Category {
        name: "freq",
        description: "CPU frequency events",
        events: &[
            Event {
                subsystem: "power",
                name: Some("cpu_frequency"),
                required: true,
            },
            Event {
                subsystem: "power",
                name: Some("clock_set_rate"),
                required: false,
            },
            Event {
                subsystem: "power",
                name: Some("clock_disable"),
                required: false,
            },
            Event {
                subsystem: "power",
                name: Some("clock_enable"),
                required: false,
            },
            Event {
                subsystem: "clk",
                name: Some("clk_set_rate"),
                required: false,
            },
            Event {
                subsystem: "clk",
                name: Some("clk_disable"),
                required: false,
            },
            Event {
                subsystem: "clk",
                name: Some("clk_enable"),
                required: false,
            },
            Event {
                subsystem: "power",
                name: Some("cpu_frequency_limits"),
                required: false,
            },
        ],
    },
    Category {
        name: "idle",
        description: "CPU idle state events",
        events: &[
            Event {
                subsystem: "power",
                name: Some("cpu_idle"),
                required: true,
            },
        ],
    },
    Category {
        name: "irq",
        description: "IRQ events",
        events: &[
            Event {
                subsystem: "irq",
                name: None,
                required: true,
            },
        ],
    },
    Category {
        name: "drm",
        description: "DRM vblank events",
        events: &[
            Event {
                subsystem: "drm",
                name: None,
                required: true,
            },
        ],
    },
    Category {
        name: "fence",
        description: "DMA-FENCE events",
        events: &[
            Event {
                subsystem: "dma_fence",
                name: None,
                required: true,
            },
            Event {
                subsystem: "sync_trace",
                name: Some("sync_timeline"),
                required: true,
            },
        ],
    },
    Category {
        name: "virtio-gpu",
        description: "virtio-gpu GPU events",
        events: &[
            Event {
                subsystem: "virtio_gpu",
                name: None,
                required: true,
            },
        ],
    },
    Category {
        name: "i915",
        description: "Intel GPU events",
        events: &[
            Event {
                subsystem: "i915",
                name: Some("i915_request_queue"),
                required: true,
            },
            Event {
                subsystem: "i915",
                name: Some("i915_request_add"),
                required: true,
            },
            Event {
                subsystem: "i915",
                name: Some("i915_request_retire"),
                required: true,
            },
            Event {
                subsystem: "i915",
                name: Some("i915_request_wait_begin"),
                required: true,
            },
            Event {
                subsystem: "i915",
                name: Some("i915_request_wait_end"),
                required: true,
            },
            Event {
                subsystem: "i915",
                name: Some("intel_gpu_freq_change"),
                required: true,
            },
            Event {
                subsystem: "i915",
                name: Some("i915_gem_evict"),
                required: true,
            },
            Event {
                subsystem: "i915",
                name: Some("i915_gem_evict_node"),
                required: true,
            },
            Event {
                subsystem: "i915",
                name: Some("i915_gem_evict_vm"),
                required: true,
            },
            Event {
                subsystem: "i915",
                name: Some("i915_gem_shrink"),
                required: true,
            },
            Event {
                subsystem: "i915",
                name: Some("i915_pipe_update_start"),
                required: true,
            },
            Event {
                subsystem: "i915",
                name: Some("i915_pipe_update_end"),
                required: true,
            },
        ],
    },
    Category {
        name: "kvm",
        description: "KVM events",
        events: &[
            Event {
                subsystem: "kvm",
                name: Some("kvm_entry"),
                required: true,
            },
            Event {
                subsystem: "kvm",
                name: Some("kvm_exit"),
                required: true,
            },
            Event {
                subsystem: "kvm",
                name: Some("kvm_userspace_exit"),
                required: true,
            },
            Event {
                subsystem: "kvm",
                name: Some("kvm_mmio"),
                required: true,
            },
            Event {
                subsystem: "kvm",
                name: Some("kvm_set_irq"),
                required: true,
            },
            Event {
                subsystem: "kvm",
                name: Some("kvm_msi_set_irq"),
                required: true,
            },
        ],
    },
    Category {
        name: "syscalls",
        description: "subsystem call events",
        events: &[
            Event {
                subsystem: "syscalls",
                name: None,
                required: true,
            },
        ],
    },
];

fn write_file(path: &Path, val: &str) -> std::io::Result<()> {
    File::create(path)?.write_all(val.as_bytes())
}

fn truncate_file(path: &Path) -> std::io::Result<()> {
    File::create(path)?;
    Ok(())
}

fn read_file(path: &Path) -> std::io::Result<String> {
    let mut val = String::new();
    File::open(path)?.read_to_string(&mut val)?;
    Ok(val)
}

fn find_tracefs() -> Option<&'static Path> {
    let tracefs_dirs = [
        "/sys/kernel/tracing",
        "/sys/kernel/debug/tracing",
    ];

    for &dir in tracefs_dirs.iter() {
        let path = Path::new(dir).join("trace");
        if path.exists() {
            return Some(Path::new(dir));
        }
    }

    None
}

fn set_buffer_size_kb(tracefs: &Path, size: u32) -> std::io::Result<()> {
    write_file(tracefs.join("buffer_size_kb").as_path(), &size.to_string())
}

fn set_trace_clock(tracefs: &Path) -> std::io::Result<()> {
    let preferred_clocks = [
        "boot",
        "mono",
        "global",
    ];

    let path = tracefs.join("trace_clock");
    let val = read_file(path.as_path())?;

    for &clock in preferred_clocks.iter() {
        if val.contains(clock) {
            // writing to trace_clock can be slow
            if val.contains(format!("[{}]", clock).as_str()) {
                return Ok(())
            } else {
                return write_file(path.as_path(), clock)
            }
        }
    }

    panic!()
}

fn set_current_tracer(tracefs: &Path, tracer: &str) -> std::io::Result<()> {
    write_file(tracefs.join("current_tracer").as_path(), tracer)
}

fn set_ftrace_filter(tracefs: &Path) -> std::io::Result<()> {
    truncate_file(tracefs.join("set_ftrace_filter").as_path())
}

fn set_option(tracefs: &Path, option: &str, val: &str) -> std::io::Result<()> {
    let mut path = tracefs.join("options");
    path.push(option);
    write_file(path.as_path(), val)
}

fn bool_to_str(val: bool) -> &'static str {
    if val { "1" } else { "0" }
}

fn set_tracing_on(tracefs: &Path, enable: bool) -> std::io::Result<()> {
    write_file(tracefs.join("tracing_on").as_path(), bool_to_str(enable))
}

fn clear_trace(tracefs: &Path) -> std::io::Result<()> {
    truncate_file(tracefs.join("trace").as_path())
}

fn dump_trace(tracefs: &Path, filename: &str) -> std::io::Result<()> {
    println!("saving the trace to {}...", filename);

    // copy does not work on CrOS
    //std::fs::copy(tracefs.join("trace").as_path(), Path::new(filename))?;
    let buf = std::fs::read(tracefs.join("trace").as_path())?;
    std::fs::write(Path::new(filename), buf)?;
    Ok(())

    /*
    let src = File::open(tracefs.join("trace").as_path())?;
    let mut dst = File::create(Path::new(filename))?;

    for line in BufReader::new(src).lines() {
        match line {
            Ok(line) => {
                // Chrome Trace does not support dma_fence
                let mut line = line.replacen(": dma_fence_", ": fence_", 1);
                line.push_str("\n");
                dst.write_all(line.as_bytes())?;
            },
            Err(_) => { break }
        }
    }

    Ok(())
    */
}

fn enable_category(tracefs: &Path, category: &Category, enable: bool) -> std::io::Result<()> {
    let mut last_missing_subsystem = None;

    for event in category.events.iter() {
        let mut path = tracefs.join("events");
        path.push(event.subsystem);

        if !path.as_path().exists() {
            if enable && (last_missing_subsystem == None ||
                          last_missing_subsystem.unwrap() != event.subsystem) {
                println!("subsystem {} is missing", event.subsystem);
                last_missing_subsystem = Some(event.subsystem);
            }
            if event.required {
                return Err(
                    std::io::Error::new(std::io::ErrorKind::NotFound, ""));
            }
            continue;
        }

        match event.name {
            Some(name) => path.push(name),
            None => (),
        }
        path.push("enable");

        match write_file(path.as_path(), bool_to_str(enable)) {
            Ok(()) => (),
            Err(err) => if enable {
                let mut pretty = event.subsystem.to_string();
                if event.name != None {
                    pretty.push_str(":");
                    pretty.push_str(event.name.unwrap());
                }

                println!("event {} is missing", pretty);

                if event.required {
                    return Err(err);
                }
            }
        }
    }

    Ok(())
}

fn main() {
    let enabled_categories : HashSet<String> = env::args().skip(1).collect();

    if enabled_categories.contains("-h") {
        println!("Usage: {} [category1] [category2]...",
                 env::args().nth(0).unwrap());
        println!("Available categories are:");

        for category in CATEGORIES.iter() {
            println!("  {}: {}", category.name, category.description);
        }
        return;
    }

    let tracefs = match find_tracefs() {
        Some(path) => path,
        None => panic!("failed to locate tracefs"),
    };

    set_option(tracefs, "overwrite", bool_to_str(true)).unwrap();
    match set_option(tracefs, "record-tgid", bool_to_str(true)) {
        Ok(_) => (),
        Err(_) => {
            // Android / CrOS only
            set_option(tracefs, "print-tgid", bool_to_str(true)).unwrap();
        },
    }

    set_buffer_size_kb(tracefs, 32 * 1024).unwrap();
    set_trace_clock(tracefs).unwrap();
    set_current_tracer(tracefs, "nop").unwrap();
    set_ftrace_filter(tracefs).unwrap();

    for category in CATEGORIES.iter() {
        let mut explicitly_enabled = None;
        if enabled_categories.is_empty() {
            explicitly_enabled = Some(false);
        } else if enabled_categories.contains(category.name) {
            explicitly_enabled = Some(true);
        }

        match explicitly_enabled {
            Some(required) => {
                match enable_category(tracefs, &category, true) {
                    Ok(_) => (),
                    Err(_) => if required {
                        panic!("failed to enable {}", category.name);
                    },
                }
            },
            None => (),
        }
    }

    set_tracing_on(tracefs, true).unwrap();
    let _ = clear_trace(tracefs);

    println!("tracing for 5 secs...");
    thread::sleep(time::Duration::from_secs(5));

    let _ = set_tracing_on(tracefs, false);
    let _ = dump_trace(tracefs, "tmp.trace");

    let _ = clear_trace(tracefs);

    for category in CATEGORIES.iter() {
        let _ = enable_category(tracefs, &category, false);
    }
}
