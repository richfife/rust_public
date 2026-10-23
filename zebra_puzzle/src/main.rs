// extern crate rand;

// use std::{process, vec};

use clap::Parser;
use rand; // Brings Rng into scope
use rand::prelude::SliceRandom;
use serde::Deserialize;
// use serde_json::Result;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
// use std::io::stdin;
// use std::path::Path;
use std::sync::Arc;
use std::thread::{self, ScopedJoinHandle};

#[derive(PartialEq)]
#[derive(Clone)]
struct Clue {
    house: usize,
    class: usize
}

// impl Clue {
//     fn clone(&self) -> Self { Self { house: self.house, class: self.class } }
// }

#[derive(Clone)]
struct CluePair {
    f: Clue,
    s: Clue
}

impl CluePair {
    fn new(h1: usize, c1: usize, h2: usize, c2: usize) -> Self { Self { f: Clue {house: h1, class: c1}, s: Clue {house: h2, class: c2}}}
}
// fn format_clue_pair(clue: &CluePair) -> String {
//     format!("(({}, {}), ({}, {}))", clue.f.house, clue.f.class, clue.s.house, clue.s.class)
// }


struct Claimed {
     claimed: Vec<Vec<bool>>
}

impl Claimed {
    fn new(classes: usize, variants: usize) -> Self {
        Self { claimed: vec![vec![false; variants]; classes] }
    }
    fn clone(&self) -> Self { Self { claimed: self.claimed.clone() } }
    fn taken(&self, class: usize, variant: usize) -> bool {self.claimed[class][variant]}
    fn claim(&mut self, class: usize, variant: usize) {self.claimed[class][variant] = true;}
}

type GuessTable = Vec<Vec<Option<usize>>>;

// fn pause() {
//     let mut _core_buffer = String::new();
//     std::io::stdin().read_line(&mut _core_buffer).expect("Failed to read line");
// }

// fn print_guess_table(guess: &GuessTable) {
//     for a in 0..guess[0].len() {
//         for h in 0..guess.len() {
//             if let Some(v) = guess[h][a] {
//                 print!("{:3}", v);
//             } else {
//                 print!("  ?");
//             }
//         }
//         println!()
//     }
// }

// Capitalize the first letter of a string.
fn uppercase_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn _solve_puzzle_impl(
    houses: usize,
    classes: usize,
    clues: &Vec<CluePair>,
    clue_idx: usize,
    previous_guess: &GuessTable,
    solution_list: &mut Vec<GuessTable>,
    claimed: &Claimed
)
{
    // If we integrated all the clues, record it as a potential solutions.
    if clue_idx >= clues.len() {
        solution_list.push((*previous_guess).clone());
        return;
    }

    let clue = &clues[clue_idx];
    let delta = clue.s.house as i32 - clue.f.house as i32;

    for h0 in 0..houses {
        let sh1 = h0 as i32 + delta;
        if sh1 < 0 || sh1 >= houses as i32 {
            continue;
        }
        let h1 = sh1 as usize;
        let a0 = clue.f.class;
        let a1 = clue.s.class;

        // Find a legal placement for this clue.
        if (previous_guess[h0][a0].is_none() && !claimed.taken(a0, clue.f.house)) || previous_guess[h0][a0].is_some_and(|v| v == clue.f.house) {
            if (previous_guess[h1][a1].is_none() && !claimed.taken(a1, clue.s.house)) || previous_guess[h1][a1].is_some_and(|v| v == clue.s.house) {
                // Create a new guess that includes it in that place.
                let mut this_guess = previous_guess.clone();
                this_guess[h0][a0] = Some(clue.f.house);
                this_guess[h1][a1] = Some(clue.s.house);

                // Mark the variants as taken.
                let mut new_claimed = (*claimed).clone();
                new_claimed.claim(a0, clue.f.house);
                new_claimed.claim(a1, clue.s.house);

                // Recurse with the next clue.
                _solve_puzzle_impl(houses, classes, clues, clue_idx + 1, &this_guess, solution_list, &new_claimed);
            }
        }
    }
}

fn solve_puzzle(
    houses: usize,
    classes: usize,
    clues: &Vec<CluePair>,
    clue_idx: usize,
    previous_guess: &GuessTable,
    solution_list: &mut Vec<GuessTable>
)

{
    // Recreate the claimed table from a previous potential solution.
    // I could keep the originals around, but this seemed simpler and
    // reasonably efficient.
    let mut claimed = Claimed::new(classes, houses);

    for h in previous_guess {
        for a in 0..h.len() {
            if h[a].is_some() {
                claimed.claim(a, h[a].unwrap());
            }
        }
    }

    // And proceed into the recursive solver.
    _solve_puzzle_impl(houses, classes, clues, clue_idx, previous_guess, solution_list, &claimed);
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Number of times to greet (optional, defaults to 1)
    #[arg(long, default_value_t = "6".to_string())]
    houses: String,

    #[arg(long, default_value_t = "6".to_string())]
    classes: String,

    #[arg(long, default_value_t = "2".to_string())]
    question_count: String,

    #[arg(long, default_value_t = String::from("config.json"))]
    config_file: String,
}

#[derive(Deserialize, Debug, Clone)]
struct ClassName {
    class: String,
    variants: Vec<String>
}

#[derive(Deserialize, Debug)]
struct Config {
    #[serde(default = "Config::default_houses")] 
    houses: usize,
    #[serde(default = "Config::default_class_count")] 
    class_count: usize,
    #[serde(default = "Config::default_question_count")] 
    question_count: usize,

    class_names: Vec<ClassName>,
}

impl Config {
    fn default_houses() -> usize {5}
    fn default_class_count() -> usize {5}
    fn default_question_count() -> usize {2}
}

fn get_arg_usize(value: &String, fallback: usize) -> usize {
    value.trim().parse::<usize>().unwrap_or(fallback)
}

fn get_config( config_file: String ) -> Result<Config, Box<dyn Error>> {
    let file = File::open(config_file)?;
    
    let reader = BufReader::new(file);

    let mut des = serde_json::Deserializer::from_reader(reader);
    let config = Config::deserialize(&mut des).unwrap();

    Ok(config)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let settings = get_config(args.config_file).unwrap();
    
    let mut item_names = settings.class_names.clone();

    let houses: usize = get_arg_usize(&args.houses, settings.houses);
    let classes: usize = get_arg_usize(&args.classes, settings.class_count);

    let question_count = get_arg_usize(&args.question_count, settings.question_count);
    let mut questions: Vec<Clue> = Vec::new();

    // We can't create more than one question from the same class or we risk
    // ambiguity.
    let mut availabe_classes: Vec<usize> = (0..classes).collect();
    availabe_classes.shuffle(&mut rand::rng());

    for _ in 0..question_count {
        // These are technically the ANSWERS (house X has item Y), but we
        // rephrase them into answers ("Which house has item Y?") when we
        // present them to the user.
        questions.push(Clue
            {
                house: rand::random_range(0..houses),
                class: availabe_classes.pop().unwrap()
            }
        );
    }

    let mut clues: Vec<CluePair> = Vec::new();

    // Start with a set of clues that will touch all of the house/class
    // combinations (except the ones that are in the questions). This usually
    // doesn't guarantee a single solution by itself, so we'll add more later.
    {
        // Initialize with shuffled stacks of classes.  We want to be able
        // to get an unused class at will, so this makes that easy.
        let mut class_stacks: Vec<Vec<usize>> = vec![vec![0; 0]; houses];

        for h in 0..class_stacks.len() {
            for c in 0..classes {
                // If a class is in a question, don't allow it here.
                if questions.contains(&Clue{house: h, class: c}) {continue;}
                class_stacks[h].push(c);
            }
            class_stacks[h].shuffle(&mut rand::rng());
        }

        // atribute_queues now contains a shuffled queue of classes
        // (3,4,2,5,1 for instance) in the order they should be used to
        // make clue pairs for that house.

        {
            // Randomly make pairs until we go 100 guesses without a usable pair.
            let give_up_point = 100;
            let mut no_match = 0;

            while no_match < give_up_point {
                // Increment the "no match" counter. If we find a usable pair, we'll reset it to 0.
                no_match += 1;

                // Pick the first house at random.  The second house will be
                // either adjacent or the same house, randomly.
                let first_house = rand::random_range(0..houses);

                // If there are no more unused classes in the first house, try again.
                if class_stacks[first_house].is_empty() {
                    continue;
                }

                let second_house: usize;
                // Emphasize adjacent houses before intra-house clues.
                if first_house > 0 && !class_stacks[first_house - 1].is_empty() {
                    second_house = first_house - 1;
                } else if first_house < houses - 1 && !class_stacks[first_house + 1].is_empty() {
                    second_house = first_house + 1;
                    // For intra-house clues, we will need two unused slots.
                } else if class_stacks[first_house].len() > 1 {
                    second_house = first_house;
                } else {
                    // No usable second house, so try again (incrementing the no_match counter).
                    continue;
                }

                // We already shuffled these, so no need to randomize here.
                let first_class = class_stacks[first_house].pop().unwrap();
                let second_class = class_stacks[second_house].pop().unwrap();

                clues.push(CluePair::new(first_house, first_class, second_house, second_class));

                // Reset the countdown counter since we found a usable pair.
                no_match = 0;
            }

            // Change any lingering items to "House X has Y" clues, since we don't
            // want to leave any classes unused.
            for h in 0..houses {
                while !class_stacks[h].is_empty() {
                    let class = class_stacks[h].pop().unwrap();
                    clues.push(CluePair::new(h, class, h, class));
                }
            }
        }
    }

    // We have a set of clues that will touch all of the house/class
    // combinations, but it may not be enough to guarantee a single solution,
    // so we'll add more clues until we do. We want to figure out which clues
    // will eliminate the most solutions, so we do some work to find them.
    let mut clue_idx = 0;
    let mut previous_guesses: Vec<GuessTable> = vec![vec![vec![None; classes]; houses]; 1];

    loop {
        let mut solution_list: Vec<GuessTable> = Vec::new();

        {
            let clues_now = clues.clone();
            let clues_arc = Arc::new(&clues_now);

            let pg_len = previous_guesses.len();
            let pg_now = previous_guesses.clone();
            let pg_arc = Arc::new(&pg_now);

            thread::scope(|s| { 
                let mut threads = Vec::<ScopedJoinHandle<'_, Vec<GuessTable>>>::new();
                for idx in 0..pg_len {
                    let clues_thread = Arc::clone(&clues_arc);
                    let pg_thread = Arc::clone(&pg_arc);
                    threads.push(s.spawn(move || {
                        let mut thread_solution_list: Vec<GuessTable> = Vec::new();
                
                        // let mut guard = vec_clone.lock().unwrap();
                        solve_puzzle(
                            houses,
                            classes,
                            &clues_thread,
                            clue_idx,
                            &pg_thread[idx],
                            &mut thread_solution_list);
                        thread_solution_list
                        }));
                }

                for thread in threads {
                    solution_list.extend(thread.join().unwrap());
                }
            });
        }

        if solution_list.len() == 0 {
            println!("No solutions found. Something went wrong.");
            return Ok(());
        }

        if solution_list.len() == 1 {
            break;
        }

        // Search for clues thet will reject the most solutions.  The goal is
        // to create an unambiguous puzzle with a few clues as possible, so we
        // want each clue to eliminate as many possibilities as possible.

        // This is a little hard to explain, but I'll try...

        // At this point, we are representing the solution as a two dimentional
        // array with house numbers across the top and the item indexes going
        // down. We have a secret, though. The indexes for each class in
        // the desired solution are always the same as the house. House #1 has
        // pet index #1, drink index #1, door index #1 and so forth.  So we
        // know at a glance that a solution that has drink index #3 in house #1
        // is wrong. But we also know another very useful thing: If a clue says
        // "The house with index 1 is right of the house with index 3", that
        // clue is wrong.  Any "is right" clue MUST have a first class 1
        // greater than the second class.

        // So, we assemble all the legal clues for each wrong solution and 
        // figure out which of them do not apply for a correct solution.  Once
        // we know which of those is most common, we eliminate it by adding a
        // correct clue it is incompatible with.

        // Create frequency map of correct clues (the inverse of wrong clues).
        let mut seen: Vec<Vec<Vec<Vec<usize>>>> = vec![vec![vec![vec![0; classes]; houses]; classes]; houses];
        for s in solution_list.iter() {
            for a1 in 0..classes {
                for a2 in 0..classes {
                    for h in 0..houses {
                        for d in 1..if h < houses - 1 {2} else {1} {
                            if let Some(l) = s[h][a1].as_ref() {
                                if let Some(r) = s[h + d][a2].as_ref() {
                                    if *r == (*l)+d {
                                        seen[*l][a1][*r][a2] += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Find the LEAST used correct clue and add it to the list, eliminating
        // the greatest number of incorrect ones.  If there is more than one at
        // the same frequency, choose one at random.
        let mut min_seen: usize = 99999;
        let mut at_seen: usize = 0;
        let mut seen_point = CluePair::new(0,0,0,0);
        for a1 in 0..classes {
            for a2 in 0..classes {
                for h in 0..houses-1 {
                    if questions.contains(&Clue {house: h, class: a1}) { continue; }
                    for d in 1..if h < houses-1 {2} else {1} {
                        if questions.contains(&Clue {house: h+d, class: a2} ) { continue; }
                        if seen[h][a1][h+d][a2] < min_seen {
                            min_seen = seen[h][a1][h+d][a2];
                            at_seen = 0;
                            seen_point = CluePair::new(h, a1, h+d, a2);
                        } else if seen[h][a1][h+d][a2] == min_seen {
                            at_seen += 1;

                            if rand::random_range(0..at_seen) == 0 {
                                seen_point = CluePair::new(h, a1, h+d, a2);
                            }
                        }
                    }
                }
            }
        }

        // Push it and loop.
        clue_idx = clues.len();

        // Keep adding clues until ambiguity is eliminated.
        // println!("{} solutions.  Adding another clue {:?}.", solution_list.len(), seen_point);

        clues.push(seen_point);
        previous_guesses = solution_list;
    }

    // One last shuffle.
    clues.shuffle(&mut rand::rng());
    for clue in clues.iter_mut() {
        if rand::random_bool(0.5) {
            // Swap the order of the clue pair half the time. So, for
            // instance, "X is left of Y" becomes "Y is right of X".
            let tmpval: CluePair = CluePair::new(clue.s.house, clue.s.class, clue.f.house, clue.f.class);
            *clue = tmpval
        }
    }
    // Shuffle the classes. It's not apparent, but the solver is actually
    // unshuffling these. The numeric representation of the street never
    // changes.
    item_names.shuffle(&mut rand::rng());
    for i in item_names.iter_mut() {
        i.variants.shuffle(&mut rand::rng());
    }

    // Iterate through the final clue list and output it.
    let mut index = 0;
    for c in clues.iter() {
        index += 1;
        print!("Clue {:2}: ", index);

        if c.f == c.s {
            println!("House number {} has {} {}.", c.f.house + 1, item_names[c.f.class].variants[c.f.house], item_names[c.f.class].class);
        } else {
            let f: &ClassName = &item_names[c.f.class];
            let s: &ClassName = &item_names[c.s.class];
            let h0 = c.f.house as i32;
            let h1 = c.s.house as i32;

            match h1 - h0 {
                -1 => println!("The house with {} {} is right of the house with {} {}.", f.variants[c.f.house], f.class, s.variants[c.s.house], s.class),
                0 => println!("The house with {} {} also has {} {}.", f.variants[c.f.house], f.class, s.variants[c.s.house], s.class),
                1 => println!("The house with {} {} is left of the house with {} {}.", f.variants[c.f.house], f.class, s.variants[c.s.house], s.class),
                i32::MIN..=-2_i32 | 2_i32..=i32::MAX => todo!(),
            }
        }
    }

    println!();
    for question in &questions {
        println!("Which house has {} {}?", item_names[question.class].variants[question.house], item_names[question.class].class);
    }

    println!();
    println!("Solution:\n---------");

    print!("{:15}", "House number");
    for h in 0..houses {
        print!("{:<11} ", h + 1);
    }
    println!();

    for a in 0..classes {
        let word = item_names[a].class.split_whitespace().last();
        match word {
            Some(last_word) => print!("{:15}", uppercase_first(last_word)),
            None => print!("{:15}", uppercase_first(&item_names[a].class))
        }
        for h in 0..houses {
            let word = item_names[a].variants[h].split_whitespace().last();
            match word {
                Some(last_word) => print!("{:11} ", uppercase_first(last_word)),
                None => print!("{:11} ", uppercase_first(&item_names[a].variants[h]))
            }
        }
        println!();
    }

    Ok(())
}