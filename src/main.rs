use std::path::Path;
use std::fs::File;
use std::io::Write;
use std::io::Read;
use std::io::BufRead;
use std::io::BufReader;
use std::{thread, time};

struct Event {
    system: &'static str,
    name: Option<&'static str>,
    required: bool,
}

struct Category {
    name: &'static str,
    description: &'static str,
    events: &'static [Event],
}

static CATEGORIES: [Category; 7] = [
    Category {
        name: "sched",
        description: "",
        events: &[
            Event {
                system: "sched",
                name: Some("sched_switch"),
                required: true,
            },
            Event {
                system: "sched",
                name: Some("sched_wakeup"),
                required: true,
            },
            Event {
                system: "sched",
                name: Some("sched_waking"),
                required: false,
            },
            // Android / CrOS only
            Event {
                system: "sched",
                name: Some("sched_blocked_reason"),
                required: false,
            },
            // Android / CrOS only
            Event {
                system: "sched",
                name: Some("sched_cpu_hotplug"),
                required: false,
            },
            Event {
                system: "sched",
                name: Some("sched_pi_setprio"),
                required: false,
            },
            Event {
                system: "cgroup",
                name: None,
                required: false,
            },
        ],
    },
    Category {
        name: "freq",
        description: "",
        events: &[
            Event {
                system: "power",
                name: Some("cpu_frequency"),
                required: true,
            },
            Event {
                system: "power",
                name: Some("clock_set_rate"),
                required: false,
            },
            Event {
                system: "power",
                name: Some("clock_disable"),
                required: false,
            },
            Event {
                system: "power",
                name: Some("clock_enable"),
                required: false,
            },
            Event {
                system: "clk",
                name: Some("clk_set_rate"),
                required: false,
            },
            Event {
                system: "clk",
                name: Some("clk_disable"),
                required: false,
            },
            Event {
                system: "clk",
                name: Some("clk_enable"),
                required: false,
            },
            Event {
                system: "power",
                name: Some("cpu_frequency_limits"),
                required: false,
            },
        ],
    },
    Category {
        name: "idle",
        description: "",
        events: &[
            Event {
                system: "power",
                name: Some("cpu_idle"),
                required: true,
            },
        ],
    },
    Category {
        name: "drm",
        description: "",
        events: &[
            Event {
                system: "drm",
                name: Some("drm_vblank_event"),
                required: true,
            },
        ],
    },
    Category {
        name: "fence",
        description: "",
        events: &[
            Event {
                system: "dma_fence",
                name: None,
                required: true,
            },
            Event {
                system: "sync_trace",
                name: Some("sync_timeline"),
                required: true,
            },
        ],
    },
    Category {
        name: "virtio-gpu",
        description: "",
        events: &[
            Event {
                system: "virtio_gpu",
                name: None,
                required: true,
            },
        ],
    },
    Category {
        name: "i915",
        description: "",
        events: &[
            Event {
                system: "i915",
                name: Some("i915_gem_ring_dispatch"),
                required: true,
            },
            Event {
                system: "i915",
                name: Some("i915_gem_ring_flush"),
                required: true,
            },
            Event {
                system: "i915",
                name: Some("i915_gem_request_add"),
                required: true,
            },
            Event {
                system: "i915",
                name: Some("i915_gem_request_complete"),
                required: true,
            },
            Event {
                system: "i915",
                name: Some("i915_gem_request_retire"),
                required: true,
            },
            Event {
                system: "i915",
                name: Some("i915_gem_request_wait_begin"),
                required: true,
            },
            Event {
                system: "i915",
                name: Some("i915_gem_request_wait_end"),
                required: true,
            },
            Event {
                system: "i915",
                name: Some("i915_gem_evict"),
                required: true,
            },
            Event {
                system: "i915",
                name: Some("i915_flip_request"),
                required: true,
            },
            Event {
                system: "i915",
                name: Some("i915_flip_complete"),
                required: true,
            },
            Event {
                system: "i915",
                name: Some("intel_gpu_freq_change"),
                required: true,
            },
        ],
    },
];

fn write_file(path: &Path, val: &str) -> std::io::Result<()> {
    File::create(path)?.write_all(val.as_bytes())
}

fn truncate_file(path: &Path) -> std::io::Result<()> {
    File::create(path);
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
    for event in category.events.iter() {
        let mut path = tracefs.join("events");
        path.push(event.system);
        match event.name {
            Some(name) => path.push(name),
            None => (),
        }
        path.push("enable");

        match write_file(path.as_path(), bool_to_str(enable)) {
            Ok(()) => (),
            Err(_) => if enable {
                let mut pretty = event.system.to_string();
                if event.name != None {
                    pretty.push_str(":");
                    pretty.push_str(event.name.unwrap());
                }

                println!("{} is missing", pretty);
            }
        }
    }

    Ok(())
}

fn main() {
    let tracefs = match find_tracefs() {
        Some(path) => path,
        None => panic!("failed to locate tracefs"),
    };

    set_option(tracefs, "overwrite", bool_to_str(true));
    set_option(tracefs, "record-tgid", bool_to_str(true));
    // Android / CrOS only
    set_option(tracefs, "print-tgid", bool_to_str(true));

    set_buffer_size_kb(tracefs, 32 * 1024);
    set_trace_clock(tracefs);
    set_current_tracer(tracefs, "nop");
    set_ftrace_filter(tracefs);

    for category in CATEGORIES.iter() {
        enable_category(tracefs, &category, true);
    }

    set_tracing_on(tracefs, true);
    clear_trace(tracefs);

    println!("tracing for 5 secs...");
    thread::sleep(time::Duration::from_secs(5));

    set_tracing_on(tracefs, false);
    dump_trace(tracefs, "tmp.trace");

    clear_trace(tracefs);

    for category in CATEGORIES.iter() {
        enable_category(tracefs, &category, false);
    }
}
