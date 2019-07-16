mod ftrace;
mod sleep;

use ftrace::Tracer;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use std::process::exit;

struct Config {
    output: PathBuf,
    timeout: u32,
    enabled_categories: Vec<usize>,
    explicit: bool,
}

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

fn usage() {
    println!("Usage: {} [options] [category1] [category2]...",
             env::args().nth(0).unwrap());

    println!();
    println!("Options");
    println!("  -h            Print this message.");
    println!("  -o <filename> Save the trace to <filename>.");
    println!("  -t <timeout>  Trace for <timeout> seconds.");

    println!();
    println!("Available categories are:");
    for cat in &CATEGORIES {
        println!("  {}: {}", cat.name, cat.description);
    }

    exit(1);
}

fn parse_args() -> Config {
    let mut config = Config {
        output: PathBuf::from("tmp.trace"),
        timeout: 5,
        enabled_categories: Vec::new(),
        explicit: false,
    };

    let mut known_categories = HashMap::new();
    for (i, cat) in CATEGORIES.iter().enumerate() {
        known_categories.insert(cat.name, i);
    }

    let mut args = env::args().skip(1);
    let mut enabled_categories = HashSet::new();
    while let Some(arg) = args.next() {
        if arg == "-h" {
            usage();
        } else if arg == "-o" {
            match args.next() {
                Some(next) => config.output = PathBuf::from(next),
                None => {
                    println!("filename is missing");
                    usage();
                }
            }
        } else if arg == "-t" {
            let mut timeout = None;
            if let Some(next) = args.next() {
                timeout = next.parse().ok();
            }

            match timeout {
                Some(timeout) => config.timeout = timeout,
                None => {
                    println!("failed to parse timeout");
                    usage();
                }
            }
        } else {
            match known_categories.get(arg.as_str()) {
                Some(index) => {
                    enabled_categories.insert(index);
                },
                None => {
                    println!("unknown category {}", arg);
                    usage();
                }
            }
        }
    }

    if enabled_categories.is_empty() {
        let all_categories = 0..CATEGORIES.len();
        config.enabled_categories.extend(all_categories);
    } else {
        config.enabled_categories.extend(enabled_categories.into_iter());
        config.explicit = true;
    }

    config
}

fn set_tracefs(tracer: &mut Tracer) {
    let tracefs_paths = [
        Path::new("/sys/kernel/tracing"),
        Path::new("/sys/kernel/debug/tracing"),
    ];

    for &tracefs in &tracefs_paths {
        tracer.set_tracefs(tracefs);
        if !tracer.has_err() {
            return;
        }
    }
}

fn set_trace_clock(tracer: &mut Tracer) {
    let preferred_clocks = [
        "boot",
        "mono",
        "global",
    ];

    let val = tracer.read("trace_clock");

    for &clock in &preferred_clocks {
        if val.contains(clock) {
            // writing to trace_clock can be slow; do not re-enable
            let active_clock = format!("[{}]", clock);
            if !val.contains(&active_clock) {
                tracer.write("trace_clock", clock);
            }
            break;
        }
    }
}

fn set_options(tracer: &mut Tracer) {
    // clear trace
    tracer.write_bool("tracing_on", false);
    tracer.truncate("trace");

    if tracer.test("options/record-tgid") {
        tracer.write_bool("options/record-tgid", true);
    } else {
        // Android / CrOS only
        tracer.write_bool("options/print-tgid", true);
    }

    tracer.write_i32("buffer_size_kb", 32 * 1024);
    tracer.write("current_tracer", "nop");
    tracer.truncate("set_ftrace_filter");

    set_trace_clock(tracer);
}

fn collect_events(tracer: &Tracer, categories: &Vec<usize>, explicit: bool) -> Vec<String> {
    let mut paths = Vec::new();
    for &index in categories {
        let cat = &CATEGORIES[index];

        let mut cat_paths = Vec::new();
        for ev in cat.events {
            let mut comps = Vec::new();
            comps.push("events");
            comps.push(ev.subsystem);
            if let Some(name) = ev.name {
                comps.push(name);
            }
            comps.push("enable");
            let path = comps.join("/");

            if explicit {
                if ev.required || tracer.test(&path) {
                    cat_paths.push(path);
                }
            } else {
                if tracer.test(&path) {
                    cat_paths.push(path);
                } else if ev.required {
                    cat_paths.clear();
                    break;
                }
            }
        }

        if cat_paths.is_empty() && !explicit {
            println!("skipping category {}", cat.name);
        }

        paths.append(&mut cat_paths);
    }

    paths
}

fn set_events(tracer: &mut Tracer, paths: &Vec<String>, enable: bool) {
    for path in paths {
        tracer.write_bool(&path, enable);
        if tracer.has_err() {
            break;
        }
    }
}

fn trace(tracer: &mut Tracer, timeout: u32) {
    tracer.write_bool("tracing_on", true);
    if !tracer.has_err() {
        sleep::sleep(timeout);
    }
    tracer.write_bool("tracing_on", false);
}

fn dump_trace(tracer: &mut Tracer, output: &Path) {
    // std::fs::copy does not work on CrOS
    let buf = tracer.read("trace");
    let _ = std::fs::write(output, buf);

    tracer.truncate("trace");
}

fn check_error(tracer: &Tracer, msg: &str) {
    if !tracer.has_err() {
        return;
    }

    let (kind, path) = tracer.get_err();
    println!("{}: {} {:?}", msg, path.to_string_lossy(), kind);
    exit(1);
}

fn main() {
    let config = parse_args();

    let mut tracer = Tracer::new();

    set_tracefs(&mut tracer);
    check_error(&tracer, "failed to set tracefs");

    println!("setting options...");
    set_options(&mut tracer);
    check_error(&tracer, "failed to set options");

    println!("setting events...");
    let event_paths = collect_events(&tracer, &config.enabled_categories, config.explicit);
    set_events(&mut tracer, &event_paths, true);
    check_error(&tracer, "failed to set events");

    println!("tracing for {} seconds...", config.timeout);
    trace(&mut tracer, config.timeout);
    check_error(&tracer, "failed to enable tracing");

    println!("saving the trace to {}...", config.output.to_string_lossy());
    dump_trace(&mut tracer, &config.output);
    check_error(&tracer, "failed to save the trace");

    // clean up
    set_events(&mut tracer, &event_paths, false);
}
