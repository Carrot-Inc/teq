// A diamond of self types 60 deep: each trait's self type names the two before it. A self type's members are its
// classes' own and their base classes' (one level: scalac finds no member of a self type's self type), so the view's
// lookup through it ends in milliseconds. Its written self definitions' positions are no pickle's (PositionPickler),
// so its judgments are listed.
trait S0 { def h0: Int = 2 }
trait S1 { self: S0 => def h1: Int = h0 }
trait S2 { self: S1 & S0 => def h2: Int = h1 }
trait S3 { self: S2 & S1 => def h3: Int = h2 }
trait S4 { self: S3 & S2 => def h4: Int = h3 }
trait S5 { self: S4 & S3 => def h5: Int = h4 }
trait S6 { self: S5 & S4 => def h6: Int = h5 }
trait S7 { self: S6 & S5 => def h7: Int = h6 }
trait S8 { self: S7 & S6 => def h8: Int = h7 }
trait S9 { self: S8 & S7 => def h9: Int = h8 }
trait S10 { self: S9 & S8 => def h10: Int = h9 }
trait S11 { self: S10 & S9 => def h11: Int = h10 }
trait S12 { self: S11 & S10 => def h12: Int = h11 }
trait S13 { self: S12 & S11 => def h13: Int = h12 }
trait S14 { self: S13 & S12 => def h14: Int = h13 }
trait S15 { self: S14 & S13 => def h15: Int = h14 }
trait S16 { self: S15 & S14 => def h16: Int = h15 }
trait S17 { self: S16 & S15 => def h17: Int = h16 }
trait S18 { self: S17 & S16 => def h18: Int = h17 }
trait S19 { self: S18 & S17 => def h19: Int = h18 }
trait S20 { self: S19 & S18 => def h20: Int = h19 }
trait S21 { self: S20 & S19 => def h21: Int = h20 }
trait S22 { self: S21 & S20 => def h22: Int = h21 }
trait S23 { self: S22 & S21 => def h23: Int = h22 }
trait S24 { self: S23 & S22 => def h24: Int = h23 }
trait S25 { self: S24 & S23 => def h25: Int = h24 }
trait S26 { self: S25 & S24 => def h26: Int = h25 }
trait S27 { self: S26 & S25 => def h27: Int = h26 }
trait S28 { self: S27 & S26 => def h28: Int = h27 }
trait S29 { self: S28 & S27 => def h29: Int = h28 }
trait S30 { self: S29 & S28 => def h30: Int = h29 }
trait S31 { self: S30 & S29 => def h31: Int = h30 }
trait S32 { self: S31 & S30 => def h32: Int = h31 }
trait S33 { self: S32 & S31 => def h33: Int = h32 }
trait S34 { self: S33 & S32 => def h34: Int = h33 }
trait S35 { self: S34 & S33 => def h35: Int = h34 }
trait S36 { self: S35 & S34 => def h36: Int = h35 }
trait S37 { self: S36 & S35 => def h37: Int = h36 }
trait S38 { self: S37 & S36 => def h38: Int = h37 }
trait S39 { self: S38 & S37 => def h39: Int = h38 }
trait S40 { self: S39 & S38 => def h40: Int = h39 }
trait S41 { self: S40 & S39 => def h41: Int = h40 }
trait S42 { self: S41 & S40 => def h42: Int = h41 }
trait S43 { self: S42 & S41 => def h43: Int = h42 }
trait S44 { self: S43 & S42 => def h44: Int = h43 }
trait S45 { self: S44 & S43 => def h45: Int = h44 }
trait S46 { self: S45 & S44 => def h46: Int = h45 }
trait S47 { self: S46 & S45 => def h47: Int = h46 }
trait S48 { self: S47 & S46 => def h48: Int = h47 }
trait S49 { self: S48 & S47 => def h49: Int = h48 }
trait S50 { self: S49 & S48 => def h50: Int = h49 }
trait S51 { self: S50 & S49 => def h51: Int = h50 }
trait S52 { self: S51 & S50 => def h52: Int = h51 }
trait S53 { self: S52 & S51 => def h53: Int = h52 }
trait S54 { self: S53 & S52 => def h54: Int = h53 }
trait S55 { self: S54 & S53 => def h55: Int = h54 }
trait S56 { self: S55 & S54 => def h56: Int = h55 }
trait S57 { self: S56 & S55 => def h57: Int = h56 }
trait S58 { self: S57 & S56 => def h58: Int = h57 }
trait S59 { self: S58 & S57 => def h59: Int = h58 }
trait S60 { self: S59 & S58 => def h60: Int = h59 }
class E extends S0 with S1 with S2 with S3 with S4 with S5 with S6 with S7 with S8 with S9 with S10 with S11 with S12 with S13 with S14 with S15 with S16 with S17 with S18 with S19 with S20 with S21 with S22 with S23 with S24 with S25 with S26 with S27 with S28 with S29 with S30 with S31 with S32 with S33 with S34 with S35 with S36 with S37 with S38 with S39 with S40 with S41 with S42 with S43 with S44 with S45 with S46 with S47 with S48 with S49 with S50 with S51 with S52 with S53 with S54 with S55 with S56 with S57 with S58 with S59 with S60 { val w = h60 }
