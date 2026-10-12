// An alias applied to a wildcard whose right-hand side refines a member as the parameter bounds
// the member by the wildcard: `Aux[? <: Opt]` is a `Cfg { type Opts <: Opt }`.
trait Opt:
  def name: String
class Verbose extends Opt:
  def name = "verbose"

trait Cfg:
  type Opts
  def opts: Opts

type Aux[O <: Opt] = Cfg { type Opts = O }

def describe(c: Aux[? <: Opt]): String = c.opts.name

@main def main(): Unit =
  val c: Aux[Verbose] = new Cfg:
    type Opts = Verbose
    def opts = Verbose()
  println(describe(c))
