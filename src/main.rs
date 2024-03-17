use std::fs::File;
use std::io::prelude::*;
use std::io;
use std::fs;
use wot_replay_parser::ReplayParser;
use serde_json::{json};

pub fn main() {
	let filtered;
	let mut input = String::new();
	let mut filter = String::new();
	println!("Replays Folder:");
	match io::stdin().read_line(&mut input) { //get folder
		Ok(_) => {
			match loop_through_files(input.trim()) { //parse files
				Ok(report) => {
					println!("Player to find:");
					match io::stdin().read_line(&mut filter) { //get filter
						Ok(_) => {
							filtered = filter_players(report, filter.trim());
							write_to_file("filtered.json", serde_json::to_string_pretty(&filtered).unwrap());
							match create_winrate_list(filtered) {													//create list
								Ok(list) => write_to_file("list.json", serde_json::to_string_pretty(&list).unwrap()),
								Err(err) => eprintln!("Error compiling list: {}", err)
							}
						}
						Err(error) => eprintln!("Error reading input: {}", error)
					}
				}
				Err(err) => eprintln!("Error finding files: {}", err)
			};
		}
		Err(error) => eprintln!("Error reading input: {}", error)
	};
	
    println!("Press Enter to exit...");
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read line");
}

pub fn create_winrate_list(data:serde_json::Value) -> Result<Vec<(String, i32, i32)>, Box<dyn std::error::Error>> {
	let mut tanks: Vec<(String, i32, i32)> = Vec::new();
	tanks.push(("All".to_string(), 0_i32, 0_i32));

	for game in data.as_array().unwrap().iter() {
		for player in game.as_array().unwrap().iter() {
			if !tanks.contains(&(player.get("tank").unwrap().as_str().unwrap().to_string(), 0_i32, 0_i32)) {
				tanks.push((player.get("tank").unwrap().as_str().unwrap().to_string(), 0_i32, 0_i32))
			}
		}
	}

	for game in data.as_array().unwrap().iter() {
		for player in game.as_array().unwrap().iter() {
			let position = tanks.iter().position(|(tank_name, _, _)| tank_name == &player.get("tank").unwrap().as_str().unwrap().to_string());
			tanks[position.unwrap()].1 += 1;
			tanks[0].1 += 1;
			if player.get("win").unwrap().as_bool().unwrap() {
				tanks[position.unwrap()].2 += 1;
				tanks[0].2 += 1;
			}
		}
	}
	
	tanks.sort_by_key(|&(_, games, _)| std::cmp::Reverse(games));
	//println!("{:?}", tanks);

	let mut text_output = String::new();
	text_output += &(format!("{:<width$}", "Tanks", width = 50) + &format!("{:<width$}", "Winrate", width = 10) + "battles \n");

	for (tank, games, wins) in &tanks {
    	let winrate = (*wins as f32 / *games as f32 * 100.0) as i32;
    	text_output += &(format!("{:<width$}", tank.to_owned(), width = 50) + &format!("{:<width$}", &(winrate.to_string() + "%"), width = 10) + &games.to_string() + "\n");
    	//text_output += &(tank.to_owned() + " " + &winrate.to_string() + "% over " + &games.to_string() + " battles \n");
	}
	println!("{}", text_output);
	Ok(tanks)
}

pub fn loop_through_files(folder: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
	let mut battles_json = json!([]);
	let paths = fs::read_dir(folder).unwrap();
	let path_count = fs::read_dir(folder).unwrap().count();

	for (i, path) in paths.enumerate() {
		let path = match path {
			Ok(entry) => entry.path(),
			Err(err) => {
				eprintln!("Error reading directory entry {}: {}", i, err);
				std::process::exit(1);
			}

		};
		print!("\rParsing file: {}/{}", i+1, path_count);
        std::io::stdout().flush().unwrap();
		//println!("file: {:?}", path);
		match get_data(&path.as_path().display().to_string()) {
			Ok(battle) => battles_json.as_array_mut().unwrap().push(battle),
			Err(err) => eprintln!(" -> Error parsing {:?}: {}", path, err)
		};
		//println!("\r");


	};
	//write_to_file("output.json", serde_json::to_string_pretty(&battles_json).unwrap());
	Ok(battles_json)
}

pub fn get_data(file: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>>  {
	let path = file.trim();
	let replay_parser;

	let mut battle_json = json!({
		"winner": "0",
		"1": [],
		"2": []
	});

	if !file.ends_with(".wotreplay") {
		//println!("{} is not a .wotreplay file", file);
		return Err(format!("not a .wotreplay file").into())
	};

	replay_parser = ReplayParser::parse_file(path).unwrap();

	if replay_parser.replay_json_end().is_none() {
		return Err(format!("Replay didn't finish.").into());
	}

	let replay_json_end = replay_parser.replay_json_end().unwrap();
	
	//let json_string_end = serde_json::to_string_pretty(&replay_json_end).unwrap();
	//write_to_file("output2.json", json_string_end);

	let win_team = replay_json_end[0]["common"]["winnerTeam"].as_i64().unwrap();
	//println!("{}", format!("{}{}", "winning team: ", win_team));
	battle_json["winner"] = json!(win_team);

	let players = replay_json_end[1].as_object().unwrap();

	for (_i, player) in players.iter().enumerate() {
		//println!("Team {}: {} in {}", player.1["team"].as_i64().unwrap(), player.1["name"].as_str().unwrap(), player.1["vehicleType"].as_str().unwrap());
		let team = player.1["team"].as_i64().unwrap() as usize;
		match player.1["team"].as_i64().unwrap(){
			1 | 2 =>{
				battle_json[format!("{}", team)].as_array_mut().unwrap().push(json!({
					"name": player.1["name"].as_str().unwrap(), "tank": player.1["vehicleType"].as_str().unwrap()
				}));
			}
			_ => return Err(format!("Couldn't find player team. Either this script is outdated, or the battle had more than 2 teams.").into())
		}
	}
	Ok(battle_json)
}

pub fn write_to_file(name:&str, json:String) {
	let mut file = match File::create(name) {
		Ok(file) => file,
		Err(err) => {
			eprintln!("Error creating file: {}", err);
			return ();
		}
	};

	match file.write_all(json.as_bytes()) {
		Ok(_) => println!("Data has been written to: {}", name),
		Err(err) => eprintln!("Error writing to file: {}", err),
	}
}

pub fn filter_players(input:serde_json::Value, filter: &str) -> serde_json::Value {
	let mut output = vec![];
	//write_to_file("filtered.json", serde_json::to_string_pretty(&input).unwrap());
	let mut count = 0;

	for battle in input.as_array().unwrap().iter() {
		count += 1;
	    let mut player_found_team = 0;
		let mut victory = false;
		
	    if let Some(team1) = battle.get("1") {
	        if team1.as_array().unwrap().iter().any(|obj| obj["name"] == filter) {
	            player_found_team = 1;
	        }
	    }
		
	    if let Some(team2) = battle.get("2") {
	        if team2.as_array().unwrap().iter().any(|obj| obj["name"] == filter) {
	            player_found_team = 2;
	        }
	    }
	    
		
	    match player_found_team {
			0 => println!("Battle {}: {} not found", count, filter),
			1 | 2 => {

				if battle.get("winner").and_then(|v| v.as_i64().map(|i| i as i32)).unwrap() == player_found_team {
					victory = true
				}
				
				let mut cloned_battle = battle.clone();
                if let Some(player) = cloned_battle[&format!("{}", player_found_team)].as_array_mut() {
                    for player_obj in player.iter_mut() {
                        player_obj.as_object_mut().unwrap().insert("win".to_string(), json!(victory));
                    }
                }
				
				let team = cloned_battle[&format!("{}", player_found_team)].clone();
				output.push(team);

				println!("Battle {}: {} found in Team {}. Victory: {}", count, filter, player_found_team, victory);
			},
			_ => println!("How did we get here?"),
		}
	}
	let mut output_json = Vec::new();

	for battle in output {
    	output_json.push(battle.clone());
	}

	return json!(output_json);
}