extern crate log;
extern crate simplelog;

use std::env;
use std::fs::File;
use std::path::Path;
use log::*;
use simplelog::*;

enum Action
{
    Buying,
    Selling,
}


struct PriceTracker
{
    // Buying bit
    buying_fee: f32, // percentage fee

    // Selling bit
    selling_fee: f32, // percentage fee

    //switch between buying and selling
    action: Action,

    // Tracking bits
    current_price: Option<f32>,
    our_price: Option<f32>,
    price_diff_percentage: Option<f32>,

    // Business decision
    //
    // The following are the 2 threasholds we want to enforce when making decisions:
    // we want to sell at a price that is at least
    // our_price + (our_price*(buying_fee + minimum_margin))
    // larly we want to buy when the price drops below
    // our_price - (our_price*(selling_fee + minimum_discount))
    minimum_margin: f32, // minimum increase we want to get before selling
    minimum_discount: f32, // minimum decrease before buying
}

fn setup(log_file: &Path)
{
    let mut logconf: ConfigBuilder = ConfigBuilder::new();
    logconf.set_location_level(LevelFilter::Error);
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Warn, logconf.build(), TerminalMode::Mixed),
            WriteLogger::new(LevelFilter::Debug, logconf.build(), File::create(log_file).unwrap())
        ]
    ).unwrap();
    
    warn!("log file {}", log_file.display());
}


impl PriceTracker
{
    fn get_price(&mut self)
    {
        self.current_price = match self.current_price
        {
            None => Some(1.0),
            Some(p) => Some(p + 0.1),
        };
        info!("current price {:?}", self.current_price);
        match self.our_price
        {
            None => {
                info!("Didn't buy anything as yet, setting price diff as 0");
                self.price_diff_percentage = None;
            }
            Some(our_price) => {
                let cur_price = self.current_price.unwrap();
                self.price_diff_percentage = Some((cur_price - our_price) / cur_price);
            }
        }
        info!("Current variation is {:?}", self.price_diff_percentage);
        // save difference in a curcular buffer, and use it to check for trends.
        // If we bought then we hold on to our assets as long as the price goes up.
        // Once the price drops we sell. The drop in price has to be confirmed X number of times before
        // we actually sell OR there is a single drop of more than Y (0.5% ?) Need to calibrate.
        //
        // If we sold we wait for the price to go below the price we paid and we wait until it drops.
        // As soon as it starts going up again (and it still is under what we paid) then we buy.
        //
        // Think about the best way to implement this algorithm.
    }
    fn ok_to_buy(&self) -> bool
    {
        info!("diff percentage {:?} < buy threshold {:?} ?", self.price_diff_percentage, self.selling_fee + self.minimum_discount);
        match self.price_diff_percentage
        {
            None => true, // When difference is an artificial 10 it means we didn't start trading as yet
            Some(d) => {
                let buy_threshold = self.selling_fee + self.minimum_discount;
                d < buy_threshold
            }
        }
    }
    fn ok_to_sell(&self) -> bool
    {
        info!("diff percentage {:?} > sell threshold {:?} ?", self.price_diff_percentage, self.buying_fee + self.minimum_margin);
        match self.price_diff_percentage
        {
            None => {
                info!("Trying to sell without difference set, impossible workflow");
                false
            }
            Some(d) => {
                let sell_threshold = self.buying_fee + self.minimum_margin;
                d > sell_threshold
            }
        }
    }

    fn buy(&mut self)
    {
        self.our_price = Some(self.current_price.unwrap() + 0.1);
        self.current_price = self.our_price;
        info!("bought at price = {:?} next action SELLING", self.our_price);
        self.action = Action::Selling;
    }

    fn sell(&mut self)
    {
        self.our_price = Some(self.current_price.unwrap() - 0.1);
        self.current_price = self.our_price;
        info!("sold at price = {:?} next action BUYING", self.our_price);
        self.action = Action::Buying;
    }
}

fn main()
{
    let args: Vec<String> = env::args().collect();

    let log_file = match args.len()
    {
        1 => {
            println!("No path passed, using default /var/log/finbot.log");
            Path::new("/var/log/finbot.log")
        },
        2 => {
            Path::new(&args[1])
        },
        _ => {
            println!("Too many arguments");
            return;
        }
    };

    setup(log_file);

    let mut decision_maker = PriceTracker
    {
        buying_fee: 0.005,
        selling_fee: 0.005,
        action: Action::Buying,
        current_price: None,
        our_price: None,
        price_diff_percentage: None,
        minimum_margin: 0.01,
        minimum_discount: 0.005,
    };

    loop
    {
        warn!("STARTING NEW CYCLE");
        decision_maker.get_price();
        match decision_maker.action
        {
            Action::Buying =>
            {
                if let true = decision_maker.ok_to_buy()
                {
                    decision_maker.buy();
                }
            },
            Action::Selling =>
            {
                if let true = decision_maker.ok_to_sell()
                {
                    decision_maker.sell();
                }
            },
        };
    }
}
