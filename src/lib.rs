pub mod engine;
pub mod utils;
pub use indicatif::ProgressBar;
pub use rand;
pub use hashbrown;


#[cfg(any(feature = "visualization", feature = "visualization_wasm", doc))]
pub mod visualization;

#[cfg(any(feature = "visualization", feature = "visualization_wasm", doc))]
pub use bevy;

pub use rand::{
    thread_rng,
    Rng,
    distributions::{Distribution, Uniform}
};

pub use csv::{Writer, Reader};
pub use std::fs::File;
pub use std::io::Write;
pub use std::fs::OpenOptions;
pub use rayon::prelude::*;
pub use std::time::Duration;

use std::error::Error;

// #[cfg(feature = "explore")]
// #[macro_use]
// extern crate memoffset;

#[cfg(feature = "explore")] 
pub use {
    memoffset::{offset_of, span_of},
    mpi::{
        datatype::UserDatatype,
        traits::*,
        Address
    },
};

#[cfg(feature = "explore")] 
pub extern crate mpi;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum Info {
    Verbose,
    Normal,
}

/**
 * 3 mode to generate the data
 * Exaustive: Brute force parameter exploration
 * Matched: explore every input with the same indexes
 * File: Read from file
 */
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExploreMode {
    Exaustive,
    Matched,
    //File,
}

/**
 * 3 mode to do model exploration
 * Local: local computation
 * Parallel: parallel computation
 * Distributed: distributed computation
 */
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ComputationMode {
    Sequential,
    Parallel,
    DistributedMPI,
}

#[macro_export]
//step = simulation step number
//states
//# of repetitions
//type of info
macro_rules! simulate {
    ($step:expr, $s:expr, $reps:expr, $info:expr) => {{
        let mut s = $s;
        let mut state = s.as_state_mut();
        let n_step: u64 = $step;

        let mut results: Vec<(Duration, f32)> = Vec::new();
        let option = $info;

        match option {
            Info::Verbose => {
                println!("\u{1F980} Rust-AB v1.0\n");
                println!(
                    "{0: >10}|{1: >9}|    {2: >11}|{3: >10}|",
                    "#Rep", "Steps", "Steps/Seconds", "Time"
                );
                println!("--------------------------------------------------");
            }
            Info::Normal => {
                println!("{esc}c", esc = 27 as char);
                println!("\u{1F980} Rust-AB v1.0\n");
                println!(
                    "{0: >10}|{1: >9}|    {2: >11}|{3: >10}|",
                    "#Rep", "Steps", "Avg. Steps/Seconds", "Avg. Time"
                );
                println!("----------------------------------------------------------------");
            }
        }
        print!("{:width$}|", 1, width = 14 - $reps.to_string().len());
        println!(
            "{:width$}|",
            n_step,
            width = 15 - n_step.to_string().len() - $reps.to_string().len()
        );
        // println!("{esc}c", esc = 27 as char);

        for r in 0..$reps {
            let mut schedule: Schedule = Schedule::new();
            state.init(&mut schedule);
            let start = std::time::Instant::now();
            let pb = ProgressBar::new(n_step);
            for i in 0..n_step {
                schedule.step(state);
                if state.end_condition(&mut schedule) {
                    break;
                }
                pb.inc(1);
            }
            pb.finish_with_message("\u{1F980}");

            let run_duration = start.elapsed();

            match option {
                Info::Verbose => {}
                Info::Normal => {
                    println!("{esc}c", esc = 27 as char);
                    println!("\u{1F980} Rust-AB v1.0\n");
                    println!(
                        "{0: >10}|{1: >9}|    {2: >11}|{3: >10}|",
                        "#Rep", "Steps", "Avg. Steps/Seconds", "Avg. Time"
                    );
                    println!("----------------------------------------------------------------");
                }
            }

            let step_seconds =
                format!("{:.0}", schedule.step as f32 / (run_duration.as_secs_f32()));
            let time = format!("{:.4}", run_duration.as_secs_f32());
            print!("{:width$}|", (r + 1), width = 14 - $reps.to_string().len());
            print!(
                "{:width$}|",
                schedule.step,
                width = 15 - n_step.to_string().len() - $reps.to_string().len()
            );
            print!("{:width$}", "", width = 13 - step_seconds.len());

            results.push((
                run_duration,
                schedule.step as f32 / (run_duration.as_nanos() as f32 * 1e-9),
            ));

            match option {
                Info::Verbose => {
                    print!("{}|", step_seconds);
                    print!("{:width$}", "", width = 9 - time.len());
                    println!("{}s|", time);
                }
                Info::Normal => {
                    let mut avg_time = 0.0;
                    let mut avg_step_seconds = 0.0;
                    for (time, step_seconds) in &results {
                        avg_time += time.as_secs_f32();
                        avg_step_seconds += step_seconds;
                    }
                    avg_time /= results.len() as f32;
                    avg_step_seconds /= results.len() as f32;
                    let avg_step_seconds = format!("{:.2}", avg_step_seconds);
                    let avg_time = format!("{:.4}", avg_time);
                    print!("{}|", avg_step_seconds);
                    print!("{:width$}", "", width = 9 - avg_time.len());
                    println!("{}s|", avg_time);
                }
            }
        }
        results
    }};
}

#[macro_use]
mod no_exported {
    #[macro_export]
    macro_rules! replace_expr {
        ($_t:tt $sub:expr) => {
            $sub
        };
    }

    //Used to count tokens of an expansion
    #[macro_export]
    macro_rules! count_tts {
        ($($tts:tt)*) => {<[()]>::len(&[$(replace_expr!($tts ())),*])};
    }

    #[macro_export]
    macro_rules! build_configurations{

        ($n_conf: expr, $( $input:ident )*) =>{{
        let mut config_table_index:Vec<Vec<usize>> = Vec::new();
        let mut input_size:usize = 0;
        let mut rep = $n_conf;
        {
            $(
                let mut row:Vec<usize> = Vec::with_capacity($n_conf);
                input_size = $input.len();
                rep /= input_size;
                let mut i = 0;
                for _ in 0..$n_conf{
                    for _ in 0..rep{
                            row.push(i);
                    }
                    i = (i + 1) % input_size;
                }
                config_table_index.push(row);
            )*
        }

        config_table_index
        }};

    }

    #[macro_export]
    macro_rules! simulate_explore {
        ($step:expr, $s:expr) => {{
            let mut s = $s;
            let mut state = s.as_state_mut();
            let n_step: u64 = $step;

            let mut results: Vec<(f32, f32)> = Vec::new();

            let mut schedule: Schedule = Schedule::new();
            state.init(&mut schedule);
            let start = std::time::Instant::now();

            for i in 0..n_step {
                schedule.step(state);

                if state.end_condition(&mut schedule) {
                    break;
                }
            }

            let run_duration = start.elapsed();

            results.push((
                run_duration.as_secs_f32(),
                schedule.step as f32 / (run_duration.as_nanos() as f32 * 1e-9),
            ));

            $s = s; // needed for model_exploration, requires also the state to be mut
            results
        }};
    }

    #[macro_export]
    ///step = simulation step number,
    ///schedule,
    ///states,
    ///input{input:type},
    ///output[output:type]
    macro_rules! explore_sequential {

        //exploration with explicit output parameters
        ($nstep: expr, $rep_conf:expr, $s:ty,
        input {$($input:ident: $input_ty: ty )*},
        output [$($output:ident: $output_ty: ty )*],
        $mode: expr,
        $( $x:expr ),* ) => {{

            //typecheck
            let _rep_conf = $rep_conf as usize;
            let _nstep = $nstep as u32;

            //println!("Calculate number of configuration");

            let mut n_conf:usize = 1;
            let mut config_table_index: Vec<Vec<usize>> = Vec::new();

            match $mode {
                ExploreMode::Exaustive =>{
                    $( n_conf *= $input.len(); )*
                    //Cartesian product with variadics, to build a table with all parameter combinations
                    //They are of different type, so i have to work with indexes
                    config_table_index = build_configurations!(n_conf, $($input )*);
                },
                ExploreMode::Matched =>{
                    $( n_conf = $input.len(); )*
                },
                // ExploreMode::File => panic!("you are not running in file mode"),
            }
            println!("n_conf {}", n_conf);

            //build_dataframe!(FrameRow, input {$( $input:$input_ty)*, }, output[ $( $output:$output_ty )*]);

            let mut dataframe: Vec<FrameRow>  = Vec::new();


            for i in 0..n_conf{
                let mut state;
                match $mode { // check which mode to use to generate the configurations
                    ExploreMode::Exaustive =>{ // use all the possible combination
                        let mut row_count = -1.;
                        state = <$s>::new(
                            $(
                            $input[config_table_index[{row_count+=1.; row_count as usize}][i]],
                            )*
                        );
                    },
                    ExploreMode::Matched =>{ // create a configuration using the combination of input with the same index
                        state = <$s>::new(
                            $(
                                $input[i],
                            )*
                        );
                    },
                    //ExploreMode::File => panic!("you are not running in file mode"),
                }

                println!("-----\nCONF {}", i);
                $(
                    println!("{}: {:?}", stringify!(state.$input), state.$input);
                )*

                for j in 0..$rep_conf{
                    println!("------\nRun {}", j+1);
                    let result = simulate_explore!($nstep, state);
                    dataframe.push( FrameRow::new(i as u32, j + 1 as u32, $(state.$input,)* $(state.$output,)* result[0].0, result[0].1, $($x,)*));
                }
            }
            dataframe
        }};

        //exploration taking default output: total time and step per second
        ($nstep: expr, $rep_conf:expr, $s:expr, input {$($input:ident: $input_ty: ty )*}, $mode:expr) => {
            explore_sequential!($nstep, $s, $rep_conf, input {$($input: $input_ty)*}, output [], $mode)
        }

    }

    #[macro_export]
    macro_rules! explore_parallel {
        ($nstep: expr, $rep_conf:expr, $s:ty,
            input {$($input:ident: $input_ty: ty )*},
            output [$($output:ident: $output_ty: ty )*],
            $mode: expr,
            $( $x:expr ),* ) => {{

            //typecheck
            let _rep_conf = $rep_conf as usize;
            let _nstep = $nstep as u32;

            //println!("Calculate number of configuration");
            let mut n_conf:usize = 1;
            let mut config_table_index: Vec<Vec<usize>> = Vec::new();

            match $mode {
                ExploreMode::Exaustive =>{
                    $( n_conf *= $input.len(); )*
                    //Cartesian product with variadics, to build a table with all parameter combinations
                    //They are of different type, so i have to work with indexes
                    config_table_index = build_configurations!(n_conf, $($input )*);
                },
                ExploreMode::Matched =>{
                    $( n_conf = $input.len(); )*
                },
                //ExploreMode::File => panic!("you are not running in file mode"),
            }
            println!("n_conf {}", n_conf);

            //build_dataframe!(FrameRow, input {$( $input:$input_ty)*, }, output[ $( $output:$output_ty )*]);

            let dataframe: Vec<FrameRow> = (0..n_conf*$rep_conf).into_par_iter().map( |run| {
                let i  = run / $rep_conf;
                /* let mut state = <$state_name>::new( $( $parameter ),*);
                let mut row_count = 0;

                $(
                    state.$input = $input[config_table_index[row_count][i]];
                    row_count+=1;
                )* */

                let mut state;
                match $mode { // check which mode to use to generate the configurations
                    ExploreMode::Exaustive =>{ // use all the possible combination
                        let mut row_count = -1.;
                        state = <$s>::new(
                            $(
                            $input[config_table_index[{row_count+=1.; row_count as usize}][i]],
                            )*
                        );
                    },
                    ExploreMode::Matched =>{ // create a configuration using the combination of input with the same index
                        state = <$s>::new(
                            $(
                                $input[i],
                            )*
                        );
                    },
                    //ExploreMode::File => panic!("you are not running in file mode"),
                }

                let result = simulate_explore!($nstep, state);
                //println!("conf {}, rep {}, run {}", i, run / n_conf, run);
                FrameRow::new(i as u32, (run % $rep_conf) as u32, $(state.$input,)* $(state.$output,)* result[0].0, result[0].1, $($x,)*)
            })
            .collect();
            dataframe
        }};


        //exploration taking default output: total time and step per second
        ($nstep: expr, $rep_conf:expr, $state_name:ty, input {$($input:ident: $input_ty: ty )*,},
        $mode: expr) => {
                explore_parallel!($nstep, $rep_conf, $state_name, input { $($input: $input_ty)*}, output [],
                $mode)
        };
    }

    #[macro_export]
    macro_rules! explore_distributed_mpi {
        ($nstep: expr, $rep_conf:expr, $s:ty,
            input {$($input:ident: $input_ty: ty )*},
            output [$($output:ident: $output_ty: ty )*],
            $mode: expr,
            $( $x:expr ),* ) => {{

            let universe = mpi::initialize().unwrap();
            let world = universe.world();
            let root_rank = 0;
            let root_process = world.process_at_rank(root_rank);
            let my_rank = world.rank();
            let num_procs = world.size() as usize;
                
            //typecheck
            let _rep_conf = $rep_conf as usize;
            let _nstep = $nstep as u32;

            let mut n_conf:usize = 1;
            let mut config_table_index: Vec<Vec<usize>> = Vec::new();
            //let mut tup: (i32, u32) = (1, 1);

            match $mode {
                ExploreMode::Exaustive =>{
                    $( n_conf *= $input.len(); )*
                    //Cartesian product with variadics, to build a table with all parameter combinations
                    //They are of different type, so i have to work with indexes
                    config_table_index = build_configurations!(n_conf, $($input )*);
                },
                ExploreMode::Matched =>{
                    $( n_conf = $input.len(); )*
                },
            }
            println!("n_conf {}", n_conf/num_procs);

            //build_dataframe!(FrameRow, input {$( $input:$input_ty)*, }, output[ $( $output:$output_ty )*]);

            let mut dataframe: Vec<FrameRow>  = Vec::new();


            for i in 0..n_conf/num_procs {
                let mut state;
                match $mode { // check which mode to use to generate the configurations
                    ExploreMode::Exaustive =>{ // use all the possible combination
                        let mut row_count = -1.;
                        state = <$s>::new(
                            $(
                            $input[config_table_index[{row_count+=1.; row_count as usize}][i*num_procs + (my_rank as usize)]],
                            )*
                        );
                    },
                    ExploreMode::Matched =>{ // create a configuration using the combination of input with the same index
                        state = <$s>::new(
                            $(
                                $input[i*num_procs + (my_rank as usize)],
                            )*
                        );
                    },
                }

                // println!("-----\nCONF {}", i);
                // $(
                //     println!("{}: {:?}", stringify!(state.$input), state.$input);
                // )*

                for j in 0..$rep_conf{
                    println!("conf {}, rep {}, pid: {}", i*num_procs + (my_rank as usize), j, my_rank);
                    let result = simulate_explore!($nstep, state);
                    dataframe.push( FrameRow::new(i as u32, j + 1 as u32, $(state.$input,)* $(state.$output,)* result[0].0, result[0].1, $($x,)*));
                }
            }

            // i have to return a dummy dataframe but i dont use it
            // only the master write the complete dataframe of all procs on csv
            if world.rank() == root_rank {
                let mut all_dataframe = vec![dataframe[0]; n_conf];
        
                root_process.gather_into_root(&dataframe[..], &mut all_dataframe[..]);
    
                all_dataframe
                
            } else {
                //every proc send to root every row
                root_process.gather_into(&dataframe[..]);
                //return dummy dataframe
                dataframe = Vec::new();
                dataframe
            }

            //dataframe
        }};


        //exploration taking default output: total time and step per second
        ($nstep: expr, $rep_conf:expr, $state_name:ty, input {$($input:ident: $input_ty: ty )*,},
        $mode: expr, 
        $( $x:expr ),* ) => {
                explore_distributed_mpi!($nstep, $rep_conf, $state_name, input { $($input: $input_ty)*}, output [],
                $mode, $( $x:expr ),*)
        };
    }
}

#[macro_export]
//macro general to call exploration
macro_rules! explore {

    //exploration with explicit output parameters
    ($nstep: expr, $rep_conf:expr, $s:ty,
    input {$($input:ident: $input_ty: ty )*},
    output [$($output:ident: $output_ty: ty )*],
    $mode: expr,
    $cmode: expr,
    $( $x:ident: $x_ty: ty ),*
    ) => {{

        // optional parameters created for distributed mode
        $(
            // it makes a new variable for optional parameters ant then it's passed
            // as an optional expression 
            let $x = $x;
        )*
        
        build_dataframe!(FrameRow, input {$( $input:$input_ty)* }, output[ $( $output:$output_ty )*], $( $x:$x_ty ),* );
        match $cmode {
            ComputationMode::Sequential => explore_sequential!($nstep, $rep_conf, $s, input {$($input: $input_ty)*}, output [$($output: $output_ty)*], $mode, $( $x ),*),
            ComputationMode::Parallel => explore_parallel!($nstep, $rep_conf, $s, input {$($input: $input_ty)*}, output [$($output: $output_ty)*], $mode, $( $x ),*),
            ComputationMode::DistributedMPI => explore_distributed_mpi!($nstep, $rep_conf, $s, input {$($input: $input_ty)*}, output [$($output: $output_ty)*], $mode, $( $x ),*),
        }
    }};


    ($nstep: expr, $rep_conf:expr, $state_name:ty, input {$($input:ident: $input_ty: ty )*,},
    $mode: expr,
    $cmode: expr) => {
                explore!($nstep, $rep_conf, $state_name, input { $($input: $input_ty)*}, output [],
                $mode, $cmode)
        };


}

///Create a csv file with the experiment results
///"DataFrame" trait allow the function to know field names and
///params list + output list for each configuration runned
pub fn write_csv<A: DataFrame>(name: &str, dataframe: &[A]) -> Result<(), Box<dyn Error>> {
    let csv_name = format!("{}.csv", name);
    let mut wtr = Writer::from_path(csv_name).unwrap();
    //define column name
    wtr.write_record(A::field_names())?;

    for row in dataframe {
        wtr.serialize(row.to_string())?;
    }

    Ok(())
}

///Trait implemented dynamically for our dataframe struct.
///We use it into "export_dataframe" function
pub trait DataFrame {
    fn field_names() -> &'static [&'static str];
    fn to_string(&self) -> Vec<String>;
}

///Generate parameter values using a Uniform Distribution
///Params: Type, Min, Max and number of samples
///n_samples is optional, can be omitted if you want a single sample
#[macro_export]
macro_rules! gen_param {
    ( $type:ty, $min:expr, $max:expr, $n:expr) => {{
        let minimum: $type;
        let maximum: $type;
        minimum = $min;
        maximum = $max;
        let mut n = $n as usize;

        //Check range parameters to avoid error with Distribution
        let (minimum, maximum) = if minimum > maximum {
            (maximum, minimum)
        } else if minimum == maximum {
            (minimum, maximum + 1 as $type)
        } else {
            (minimum, maximum)
        };

        if n == 0 {
            n = 1;
        }

        let between = Uniform::from(minimum..maximum);
        let mut rng = rand::thread_rng();
        let dist: Vec<$type> = between.sample_iter(&mut rng).take($n).collect();

        dist
    }};

    //gen a single value
    (  $type:ty, $min:expr, $max:expr) => {{
        gen_param!($type, $min, $max, 1)
    }};
}

#[macro_export]
macro_rules! build_dataframe {
    //Dataframe with input and output parameters

    //Dataframe with input and output parameters and optional parameters
    ($name:ident, input {$($input: ident: $input_ty: ty)*}, output [$($output: ident: $output_ty: ty)*], $( $x:ident: $x_ty: ty ),*) => {

        #[derive(Default, Clone, Copy, PartialEq, Debug)]
        struct $name {
            pub conf_num: u32,
            pub conf_rep: u32,
            $(pub $input: $input_ty,)*
            $(pub $output: $output_ty,)*
            pub run_duration: f32,
            pub step_per_sec: f32,
            $(pub $x: $x_ty,)*
        }

        unsafe impl Equivalence for $name {
            type Out = UserDatatype;
            
            fn equivalent_datatype() -> Self::Out {

                //count input and output parameters to create slice for blocklen
                let v_in = count_tts!($($input)*);
                let v_out = count_tts!($($output)*);
                let v_x = count_tts!($($x)*);

                let dim = v_in + v_out + v_x + 4;
                let mut vec = Vec::with_capacity(dim);
                for i in 0..dim {
                    vec.push(1);
                }
                //UserDatatype::structured(blocklengths: &[Count], displacements: &[Address], types: &[D])
                UserDatatype::structured(    
                    vec.as_slice(), 
                    &[
                        offset_of!($name, conf_num) as Address,
                        offset_of!($name, conf_rep) as Address,
                        $(
                            offset_of!($name, $input) as Address,
                        )*
                        $(
                            offset_of!($name, $output) as Address,
                        )*
                        offset_of!($name, run_duration) as Address,
                        offset_of!($name, step_per_sec) as Address,
                        $(
                            offset_of!($name, $x) as Address,
                        )*
                    ],
                    &[
                        u32::equivalent_datatype(),
                        u32::equivalent_datatype(),
                        $(
                            <$input_ty>::equivalent_datatype(),
                        )*
                        $(
                            <$output_ty>::equivalent_datatype(),
                        )*
                        f32::equivalent_datatype(),
                        f32::equivalent_datatype(),
                        $(
                            <$x_ty>::equivalent_datatype(),
                        )*
                    ]
                )
            }
        }

        impl DataFrame for $name{
            fn field_names() -> &'static [&'static str] {
                static NAMES: &'static [&'static str] = &["Simulation", "Run", $(stringify!($input),)* $(stringify!($output),)*  "Run Duration", "Step per sec", $(stringify!($x),)*];
                NAMES
            }

            fn to_string(&self) -> Vec<String> {
                let mut v: Vec<String> = Vec::new();
                v.push(self.conf_num.to_string());
                v.push(self.conf_rep.to_string());
                $(
                    v.push(format!("{:?}", self.$input));
                )*
                $(
                    v.push(format!("{:?}", self.$output));
                )*
                v.push(self.run_duration.to_string());
                v.push(self.step_per_sec.to_string());
                $(
                    v.push(format!("{:?}", self.$x));
                )*
                v
            }

        }

        impl $name {
            pub fn new( conf_num: u32, conf_rep: u32 $(, $input: $input_ty)* $(, $output: $output_ty)*, run_duration: f32, step_per_sec: f32 $(, $x: $x_ty)*,) -> $name{
                $name {
                    conf_num,
                    conf_rep,
                    $(
                        $input,
                    )*
                    $(
                        $output,
                    )*
                    run_duration,
                    step_per_sec,
                    $(
                        $x,
                    )*
                }
            }
        }
    };

    //Dataframe with only input parameters
    ($name:ident, input{$($element: ident: $input_ty: ty)* }) => {
        build_dataframe!($name, input{$($element: $input_ty)*}, output[]);
    };

    //Dataframe with only input parameters
    ($name:ident, input {$($input: ident: $input_ty: ty)*}, $( $x:ident: $x_ty: ty ),*) => {
        build_dataframe!($name, input{$($element: $input_ty)*}, output[], $( $x:ident: $x_ty: ty ),*);
    };
}

#[macro_export]
macro_rules! load_csv {

    ($input_file: expr, $( $x:ident: $x_ty: ty ),*) =>{{
        
        let mut rdr = Reader::from_path($input_file).unwrap();
        $(
            let mut $x: Vec<$x_ty> = Vec::new();
        )*
        for result in rdr.records() {
            let record = result.unwrap();
            let mut i = 0;
            $(
                let x : $x_ty = record[i].parse().unwrap();
                $x.push(x);
                i += 1;
            )*
        }
        let v = ($( $x, )*);
        v
    }};
}
        
#[macro_export]
macro_rules! ga{
    (
        $init_population:tt,
        $fitness:tt,
        $selection:tt,
        $mutation:tt,
        $crossover:tt,
        $state: ty,
        $desired_fitness: expr,
        $generation_num: expr,
        $step: expr
    ) => {
        
        let mut population: Vec<$state> = $init_population();

        // let mut index = 0;
        
        let mut generation = 0;
        // calculate the fitness for the first population
        loop {
            // if generation_num is passed as 0, we have infinite generations
            if $generation_num != 0 && generation == $generation_num {
                println!("Reached the desired generations number, exiting...");
                break;
            }
            
            generation += 1;
            println!("Computing generation {}...", generation);
            // execute the simulation for each member of population
            let mut fitness_values: Vec<f32> = Vec::new();

            for individual in population.iter_mut() {
                let mut schedule: Schedule = Schedule::new();
                individual.init(&mut schedule);
                
                for _ in 0..($step as usize) {
                    let individual = individual.as_state_mut();
                    schedule.step(individual);
                    if individual.end_condition(&mut schedule) {
                        break;
                    }
                }
                
                fitness_values.push($fitness(individual, schedule));
            }

            let mut file = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .append(true)
                        .create(true)
                        .open("ga_result.txt").unwrap();

            let _ = writeln!(&mut file, "Population at gen {}", generation).unwrap();
            for i in 0..population.len(){
                let _ = writeln!(&mut file, "- individual #{}: {} ", i, population[i]);
            }
            let _ = writeln!(&mut file, "------------------------------");

            let mut flag = false;
            for i in 0..fitness_values.len(){
                if fitness_values[i] >= $desired_fitness{
                    flag = true;
                    // index = i;
                    break;
                }
                
            }

            if flag {
                break;
            }

            // compute selection
            $selection(&mut population);
            
            if population.len() <= 1 {
                println!("Population size <= 1, exiting...");
                break;
            }

            // mutate the new population
            for individual in population.iter_mut() {
                $mutation(individual);
            }
            
            // crossover the new population
            $crossover(&mut population);
        }

        population.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());

        println!("Best population is found at generation {}:", generation);
        //for i in 0..population.len(){
        println!("The best individual is: {} ", population[0]);
        //}
    };
}