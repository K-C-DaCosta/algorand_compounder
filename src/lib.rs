use algo_rust_sdk::AlgodClient;
use std::{error, error::Error, fmt};

pub trait Evaluate1D {
    ///evaluate f(x)
    fn eval(&self, x: f64) -> f64;

    /// evaluate f'(x) , where `delta`->0+
    /// uses central differences to approximate derivative
    fn first_derivative(&self, x: f64, delta: f64) -> f64 {
        (self.eval(x + delta) - self.eval(x - delta)) / (2.0 * delta)
    }

    /// evaluate f''(x), where `delta` -> 0+
    /// uses a finite difference approximation
    fn second_derivative(&self, x: f64, delta: f64) -> f64 {
        (self.eval(x + delta) - (2.0 * self.eval(x)) + self.eval(x - delta)) / (delta * delta)
    }
    /// # Description
    /// finds extrema(approximately) using newtons method
    /// # Parameters
    /// * `x0` - the initial guess
    /// * `delta` - the value for finite differences
    /// * `epsilon` - the error threshhold, the closer to zero the more accurate
    fn search_extrema_newton(
        &self,
        mut x0: f64,
        max_iters: usize,
        delta: f64,
        epsilon: f64,
    ) -> Option<f64> {
        for _ in 0..max_iters {
            x0 = x0 - (self.first_derivative(x0, delta) / self.second_derivative(x0, delta));
            if self.first_derivative(x0, delta).abs() < epsilon {
                return Some(x0);
            }
        }
        None
    }

    /// # Description
    /// finds extrema(approximately) using bisection method
    /// # Parameters
    /// * `range` - the range for which the extrema may potentially lie
    /// * `delta` - the value for finite differences
    /// * `epsilon` - the error threshhold, the closer to zero the more accurate
    fn search_extrema_bisection(
        &self,
        mut range: (f64, f64),
        max_iters: usize,
        delta: f64,
        epsilon: f64,
    ) -> Option<f64> {
        for _ in 0..max_iters {
            let (l, u) = range;
            let mid = (u - l) * 0.5 + l;
            let fl = self.first_derivative(l, delta);
            let fm = self.first_derivative(mid, delta);
            let fu = self.first_derivative(u, delta);
            let is_fl_pos = fl > 0.;
            let is_fu_pos = fu > 0.;
            let is_fm_pos = fm > 0.;

            if is_fl_pos == is_fu_pos {
                return None;
            } else if fm.abs() < epsilon {
                return Some(mid);
            } else if is_fm_pos != is_fu_pos {
                range = (mid, u);
            } else if is_fl_pos != is_fm_pos {
                range = (l, mid);
            } else {
                return None;
            }
        }

        let (l, u) = range;
        Some((u - l) * 0.5 + l)
    }
}

pub trait Coefs {}

/// # Description
/// A structure representing a simple,1d, closed-form, analytic function
pub struct Function1DAnalytic<FuncType, CoefsType> {
    pub func: FuncType,
    pub coefs: CoefsType,
}

impl<FuncType, CoefsType> Function1DAnalytic<FuncType, CoefsType>
where
    FuncType: Fn(f64, CoefsType) -> f64 + Copy,
    CoefsType: Coefs + Copy,
{
    ///define and create f(x)
    pub fn new(func: FuncType, coefs: CoefsType) -> Self {
        Self { func, coefs }
    }
}

#[derive(Copy, Clone)]
pub struct CompoundModelCoefs {
    pub years: f64,
    pub rate: f64,
    pub avg_fees: f64,
    pub initial_principal: f64,
}
impl CompoundModelCoefs {
    pub fn new(years: f64, rate: f64, avg_fees: f64, initial_principal: f64) -> Self {
        Self {
            years,
            rate,
            avg_fees,
            initial_principal,
        }
    }
}

impl Coefs for CompoundModelCoefs {}

pub struct AlgoInterestModel {
    model: Function1DAnalytic<fn(f64, CompoundModelCoefs) -> f64, CompoundModelCoefs>,
}
impl AlgoInterestModel {
    pub fn new(coefs: CompoundModelCoefs) -> Self {
        Self {
            model: Function1DAnalytic::new(Self::projected_wallet_price, coefs),
        }
    }

    /// # Description
    /// returns the optimal number of seconds you should wait before collecting the reward
    pub fn get_ideal_reward_wait_time(&self) -> Option<f64> {
        self.search_extrema_bisection((1.0, 1000000000.), 64, 0.0001, 0.0000001)
            .map(|optimal_collections_per_year| {
                (365.0 / optimal_collections_per_year) * 24.0 * 3600.
            })
    }

    /// # Description
    /// models the balance of the wallet as a function of the the collection rate(per year)
    /// # Comments
    /// * takes into account fees and compounding
    /// # Model Derivation
    /// First i came up with a recurrence relation for compounding:
    /// ```
    ///  C(0) = A
    ///  C(n) = C(n-1)*(1 + r/t )^t - f*t
    /// ```
    /// * `A` - is principal
    /// * `r` - is interest rate per year
    /// * `t` - is number of time you collect rewards per year
    /// * `f` - is average fee per collection
    /// * `n` - is the years of compounding \
    /// Solving the recurrence relation yields a closed-form equation, which you can then use to find the optimal 't'.
    /// I use simple numerical approximations for finding the local extrema of the function  
    fn projected_wallet_price(collections_per_year: f64, coefs: CompoundModelCoefs) -> f64 {
        // 'g' is a sub expression in the complete formula that appears multiple times.
        // I have no meaningful name to give it
        let g = (coefs.rate / collections_per_year + 1.0).powf(collections_per_year);
        coefs.initial_principal * g.powf(coefs.years)
            - ((collections_per_year * coefs.avg_fees) * (g.powf(coefs.years) - 1.0)) / (g - 1.0)
    }
}

impl Evaluate1D for AlgoInterestModel {
    fn eval(&self, x: f64) -> f64 {
        (self.model.func)(x, self.model.coefs)
    }
}

#[derive(Debug)]
pub struct ConfirmationError {
    msg: String,
}

impl ConfirmationError {
    pub fn new(msg: String) -> Box<Self> {
        Box::new(Self { msg })
    }
}

impl fmt::Display for ConfirmationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl error::Error for ConfirmationError {}

pub fn print_algod_status(algod_client: &AlgodClient) -> Result<(), Box<dyn Error>> {
    let node_status = algod_client.status()?;
    println!("algod last round: {}", node_status.last_round);
    println!(
        "algod time since last round: {}",
        node_status.time_since_last_round
    );
    println!("algod catchup: {}", node_status.catchup_time);
    println!("algod latest version: {}", node_status.last_version);
    Ok(())
}
