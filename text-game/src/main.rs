use rusty_ecs_core::{Entity, World, System, SystemExecutor};
use std::io::{self, Write};

// Components
#[derive(Clone, Copy)]
struct Name(&'static str);

#[derive(Clone, Copy)]
struct Health {
    pub hp: i32,
    pub max: i32,
}

#[derive(Clone, Copy)]
struct Damage {
    pub value: i32,
}

#[derive(Clone, Copy, Default)]
struct Defending(pub bool);

#[derive(Clone, Copy)]
struct Player;

#[derive(Clone, Copy)]
struct Enemy;

// Events
struct AttackEvent {
    pub attacker: Entity,
    pub target: Entity,
    pub damage: i32,
}

// Systems
struct DamageSystem;

impl System for DamageSystem {
    fn run(&mut self, world: &mut World) {
        let attacks = world.take_events::<AttackEvent>();
        for attack in attacks {
            let mut damage = attack.damage;
            if is_defending(world, attack.target) {
                damage = (damage / 2).max(0);
            }

            let target_name = world
                .get_component::<Name>(attack.target)
                .map(|n| n.0)
                .unwrap_or("Unknown");
            let attacker_name = world
                .get_component::<Name>(attack.attacker)
                .map(|n| n.0)
                .unwrap_or("Unknown");
            let attacker_is_player = world.get_component::<Player>(attack.attacker).is_some();

            if let Some(h) = world.get_component_mut::<Health>(attack.target) {
                h.hp = (h.hp - damage).max(0);

                if attacker_is_player {
                    println!(
                        "You strike {} for {} damage! (HP: {}/{})",
                        target_name, damage, h.hp, h.max
                    );
                } else {
                    println!(
                        "{} hits you for {} damage! (HP: {}/{})",
                        attacker_name, damage, h.hp, h.max
                    );
                }
            }
        }
    }
}

fn main() {
    println!("Welcome to Rusty Text Battle!\n");

    let mut world = World::new();

    let player = world.create_entity();
    world.add_component(player, Name("Hero"));
    world.add_component(player, Player);
    world.add_component(player, Health { hp: 45, max: 45 });
    world.add_component(player, Damage { value: 7 });
    world.add_component(player, Defending(false));

    let enemies_data = vec![
        ("Goblin", 12, 3, vec!["Slash", "Bite"]),
        ("Orc", 18, 5, vec!["Heavy Swing", "Headbutt"]),
        ("Necromancer", 22, 6, vec!["Shadow Bolt", "Bone Spike"]),
    ];

    let mut enemy_entities: Vec<Entity> = Vec::new();
    for (name, hp, dmg, _attacks) in &enemies_data {
        let e = world.create_entity();
        world.add_component(e, Name(*name));
        world.add_component(e, Enemy);
        world.add_component(e, Health { hp: *hp, max: *hp });
        world.add_component(e, Damage { value: *dmg });
        enemy_entities.push(e);
    }

    let mut executor = SystemExecutor::new();
    executor.add_system(DamageSystem);

    let mut current_enemy_index = 0usize;

    loop {
        let player_alive = world
            .get_component::<Health>(player)
            .map(|h| h.hp > 0)
            .unwrap_or(false);
        if !player_alive {
            println!("You have fallen. Game Over.");
            break;
        }

        if current_enemy_index >= enemy_entities.len() {
            println!("All enemies are defeated! You win!");
            break;
        }

        let enemy = enemy_entities[current_enemy_index];

        let enemy_alive = world
            .get_component::<Health>(enemy)
            .map(|h| h.hp > 0)
            .unwrap_or(false);
        if !enemy_alive {
            println!(
                "{} has been defeated!",
                world.get_component::<Name>(enemy).unwrap().0
            );
            current_enemy_index += 1;
            continue;
        }

        let en_name = world.get_component::<Name>(enemy).unwrap().0;
        let attacks = &enemies_data[current_enemy_index].3;
        println!("An enemy approaches: {}", en_name);
        println!("It brandishes these attacks: {}\n", attacks.join(", "));

        let p_hp = world.get_component::<Health>(player).unwrap();
        let e_hp = world.get_component::<Health>(enemy).unwrap();
        println!(
            "Status => You: {}/{} | {}: {}/{}",
            p_hp.hp, p_hp.max, en_name, e_hp.hp, e_hp.max
        );

        set_defending(&mut world, player, false);
        let action = prompt_player_action();
        match action.as_str() {
            "attack" | "a" => {
                let dmg = world.get_component::<Damage>(player).unwrap().value;
                world.push_event(AttackEvent {
                    attacker: player,
                    target: enemy,
                    damage: dmg,
                });
            }
            "defend" | "d" => {
                set_defending(&mut world, player, true);
                println!("You brace yourself, reducing incoming damage this turn!");
            }
            "quit" | "q" => {
                println!("You chose to retreat. Game Over.");
                break;
            }
            _ => {
                println!("Unrecognized action. You hesitate and lose your turn!");
            }
        }

        // Run systems to process player's attack
        executor.run(&mut world);

        let enemy_alive = world
            .get_component::<Health>(enemy)
            .map(|h| h.hp > 0)
            .unwrap_or(false);
        
        if !enemy_alive {
            println!("{} collapses!", en_name);
            continue;
        }

        // Enemy turn
        let enemy_attack_name = &enemies_data[current_enemy_index].3[rand_index(attacks.len())];
        let enemy_damage = world.get_component::<Damage>(enemy).unwrap().value;
        
        println!("{} uses {}!", en_name, enemy_attack_name);
        world.push_event(AttackEvent {
            attacker: enemy,
            target: player,
            damage: enemy_damage,
        });

        // Run systems to process enemy's attack
        executor.run(&mut world);
        println!();
    }

    println!("Thanks for playing!");
}

fn prompt_player_action() -> String {
    print!("Choose action [attack(a)/defend(d)/quit(q)]: ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        input = input.trim().to_lowercase();
    }
    input
}

fn set_defending(world: &mut World, entity: Entity, value: bool) {
    if let Some(d) = world.get_component_mut::<Defending>(entity) {
        d.0 = value;
    }
}

fn is_defending(world: &World, entity: Entity) -> bool {
    world
        .get_component::<Defending>(entity)
        .map(|d| d.0)
        .unwrap_or(false)
}

fn rand_index(n: usize) -> usize {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let seed = now.as_nanos() as u64;
    (seed as usize) % n
}
