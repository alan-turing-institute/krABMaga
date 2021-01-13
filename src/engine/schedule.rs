extern crate priority_queue;

use crate::engine::agent::Agent;
use crate::engine::agentimpl::AgentImpl;
use crate::engine::priority::Priority;
use priority_queue::PriorityQueue;
use crate::engine::state::State;

pub struct Schedule<A: 'static + Agent + Clone + Send> {
    pub step: usize,
    pub time: f64,
    pub events: PriorityQueue<AgentImpl<A>, Priority>,
}

#[derive(Clone)]
pub struct Pair<A: 'static + Agent + Clone> {
    agentimpl: AgentImpl<A>,
    priority: Priority,
}

impl<A: 'static + Agent + Clone> Pair<A> {
    fn new(agent: AgentImpl<A>, the_priority: Priority) -> Pair<A> {
        Pair {
            agentimpl: agent,
            priority: the_priority,
        }
    }
}

impl<A: 'static + Agent + Clone + Send> Schedule<A> {
    pub fn new() -> Schedule<A> {
        //println!("Sequential schedule");
        Schedule {
            step: 0,
            time: 0.0,
            events: PriorityQueue::new(),
        }
    }

    pub fn schedule_once(&mut self, agent: AgentImpl<A>, the_time: f64, the_ordering: i64) {
        self.events.push(
            agent,
            Priority {
                time: the_time,
                ordering: the_ordering,
            },
        );
    }

    pub fn schedule_repeating(&mut self, agent: A, the_time: f64, the_ordering: i64) {
        let mut a = AgentImpl::new(agent);
        a.repeating = true;
        let pr = Priority::new(the_time, the_ordering);
        self.events.push(a, pr);
    }

    pub fn step(&mut self,state: &mut <A as Agent>::SimState){
        if self.step == 0{
            state.update();
        }
        self.step += 1;


        // let start: std::time::Instant = std::time::Instant::now();
        let events = &mut self.events;
        if events.is_empty() {
            println!("coda eventi vuota");
            return;
        }

        let mut cevents: Vec<Pair<A>> = Vec::new();

        match events.peek() {
            Some(item) => {
                let (_agent, priority) = item;
                self.time = priority.time;
            }
            None => panic!("agente non trovato"),
        }

        loop {
            if events.is_empty() {
                break;
            }

            match events.peek() {
                Some(item) => {
                    let (_agent, priority) = item;
                    if priority.time > self.time {
                        break;
                    }
                }
                None => panic!("agente non trovato"),
            }

            let item = events.pop();
            match item {
                Some(item) => {
                    let (agent, priority) = item;
                    // let x = agent.id.clone();
                    // println!("{}", x);
                    cevents.push(Pair::new(agent, priority));
                }
                None => panic!("no item"),
            }
        }
       
        for mut item in cevents.into_iter() {
            item.agentimpl.agent.step(state);
            if item.agentimpl.repeating {
                self.schedule_once(
                    item.agentimpl,
                    item.priority.time + 1.0,
                    item.priority.ordering,
                );
            }
        }
        
        state.update();
        // println!("Time spent calling step method, step {} : {:?}",self.step,start.elapsed());

    }
}