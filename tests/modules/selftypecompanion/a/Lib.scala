package lib

// A case class nested in a trait, its companion the one `Desugar.classDef` makes, created from
// another trait through its self type: `this` is a `Parser & Base`, whose part `Base` declares
// the class (monocle's `FocusBase.LambdaConfig` built in `LambdaConfigParser`), which scalac's
// pickle selects on `Parser.this` (`Parser.this.Cfg.apply(n, 1)`).
trait Base:
  case class Cfg(name: String, body: Int)

trait Parser:
  this: Base =>
  def make(n: String): Any = this.Cfg(n, n.length)

class Impl(val k: Int) extends Base with Parser:
  def use(n: String): Int = make(n) match
    case Cfg(_, b) => b + k
    case _ => 0
