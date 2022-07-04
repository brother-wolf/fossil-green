extern crate rusoto_ce;
extern crate serde_json;
extern crate serde;

use ::rusoto_ce::{CostExplorer,GetCostAndUsageRequest,DateInterval,GroupDefinition,ResultByTime, Group};
use structopt::StructOpt;
// use itertools::Itertools;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
// use serde_json::Result;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Cost {
    pub date: String,
    pub region: String,
    pub cost: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct FuelCostByDate {
    pub date: String,
    pub fuel_cost: FuelCost,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct FuelCost {
    pub green: f64,
    pub grey: f64,
}


#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
struct Opt {
    #[structopt(short = "a", long = "aws-profile")]
    aws_profile: String,
    #[structopt(short = "s", long = "start-date")]
    start: String,
    #[structopt(short = "e", long = "end-date")]
    end: String,
}


fn get_blended_cost(group: &Group) -> f64 {
    let cost = group.metrics.as_ref().unwrap().get("BlendedCost").unwrap().amount.as_ref().unwrap().parse::<f64>().unwrap();
    // println!("{:?}", cost);
    cost
}

fn get_region(group: &Group) -> Option<&String> {
    group.keys.as_ref().unwrap().first()
}

fn is_service(group: &Group, excluded_services: &Vec<String>) -> bool {
    match group.keys.as_ref().unwrap().get(1) {
        Some(service) => !excluded_services.contains(service),
        None => false,
    }
}

fn get_results_time(result_by_time: &ResultByTime) -> String {
    result_by_time.time_period.as_ref().unwrap().start.clone()
    // Dates { start: , end: result_by_time.time_period.as_ref().unwrap().end.clone()}
}

fn main() {
    let opt = Opt::from_args();
    // let title = if opt.name.is_empty() { opt.aws_profile.clone() } else { opt.name };
    
    let green_regions = vec!["us-west-2".to_string(), "eu-central-1".to_string(), "eu-west-1".to_string(), "ca-central-1".to_string(), "us-gov-west-1".to_string()];
    let excluded_services = vec!["Refund".to_string(), "Tax".to_string()];

    let start_date = opt.start;
    let end_date = opt.end;

    let rt = Runtime::new().unwrap();

    match aws_connections_lib::cost_explorer::get_client(&opt.aws_profile, "us-east-1") {
        
        Ok(client) => {
            let interval = DateInterval {
                start: start_date,
                end: end_date,
            };
            let group_by_region = GroupDefinition{key: Some("REGION".to_string()), type_: Some("DIMENSION".to_string())};
            let group_by_service = GroupDefinition{key: Some("SERVICE".to_string()), type_: Some("DIMENSION".to_string())};
            let metrics = vec!["BlendedCost".to_string(), "UsageQuantity".to_string()];
            let cost_and_usage_request = GetCostAndUsageRequest { 
                filter: None, 
                group_by: Some(vec![group_by_region, group_by_service]), 
                granularity: Some("MONTHLY".to_string()), 
                metrics: Some(metrics), 
                next_page_token: None, 
                time_period: interval,
            };

            let mut costs_by_region = match rt.block_on(async { client.get_cost_and_usage(cost_and_usage_request).await }){
            // let mut costs_by_region = match client.get_cost_and_usage(cost_and_usage_request).sync() {
                Err(_e) => {
                    println!("{:?}", _e);
                    vec![]
                },
                Ok(cost_and_usage) => {
                    cost_and_usage.results_by_time.unwrap().iter().flat_map(|result_by_time| 

                        match &result_by_time.groups {
                            None => vec![],
                            Some(groups) => groups.iter().flat_map(|group|                             
                                if is_service(group, &excluded_services) {
                                    match get_region(group) {
                                        Some(region) => Some(Cost{
                                            date: get_results_time(result_by_time),
                                            region: region.clone(), 
                                            cost: get_blended_cost(group) 
                                        }),
                                        None => None,
                                    }
                                } else { None }
                            ).collect()
                        }
                        
                    ).collect()
                }
            };

            costs_by_region.sort_by(|d1, d2| d1.date.cmp(&d2.date));
            
            let green_grey_costs_by_month = costs_by_region.iter().map(|cost_by_region| {
                if green_regions.contains(&cost_by_region.region) {
                    FuelCostByDate {date: cost_by_region.date.clone(), fuel_cost: FuelCost { green: cost_by_region.cost, grey: 0.0,}, }
                } else {
                    FuelCostByDate {date: cost_by_region.date.clone(), fuel_cost: FuelCost{ green: 0.0, grey: cost_by_region.cost, }, }
                }
            });
            let mut green_grey_costs_grouped_by_month: HashMap<String, Vec<FuelCost>> = HashMap::new();
            green_grey_costs_by_month.into_iter().for_each(|fc| {
                let group = green_grey_costs_grouped_by_month.entry(fc.date.clone()).or_insert(vec![]);
                group.push(fc.fuel_cost);
            });
            
            let monthly_costs: HashMap<String, (f64, f64)> = green_grey_costs_grouped_by_month.iter().map(|(key, fuel_costs)| 
            (key.clone(), fuel_costs.iter().fold( (0.0, 0.0), | (green, grey), fuel_cost | (green + fuel_cost.green, grey + fuel_cost.grey)))).collect();
            let mut results: Vec<FuelCostByDate> = monthly_costs.iter().map(|(key, (green, grey))| FuelCostByDate{date: key.clone(), fuel_cost: FuelCost { green: green.clone(), grey: grey.clone()}}).collect::<Vec<FuelCostByDate>>();
            let (total_green, total_grey) = results.iter().fold(( 0.0, 0.0), | (green, grey), fuel_cost_by_date | (green + fuel_cost_by_date.fuel_cost.green, grey + fuel_cost_by_date.fuel_cost.grey));
            let total = FuelCostByDate{date: "total".to_string(), fuel_cost: FuelCost{ green: total_green, grey: total_grey}};
            results.push(total);
            results.sort_by(|a, b| b.date.cmp(&a.date));

            let json = match serde_json::to_string(&results) {
                Ok(json) => json,
                Err(_e) => "[]".to_string(),
            };
            println!("{:?}", &json);
        },
        Err(e) => println!("unable to establish credentials, {}", e.message),
    };
}
