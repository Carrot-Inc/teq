// Path equality and subtyping between type members, each answer from scalac 3.8.4: the lines
// without an error are what scalac accepts.
// expect: 39:23: error: type mismatch: found b.Elem, required a.Elem
// expect: 42:25: error: type mismatch: found Container#Elem, required a.Elem
// expect: 45:21: error: type mismatch: found a.Elem, required Int
// expect: 47:31: error: type mismatch: found (fa : a.F[Int]), required a.F[String]
// expect: 48:31: error: type mismatch: found (fb : b.F[Int]), required a.F[Int]
// expect: 50:51: error: type mismatch: found Names, required Container{type Elem = Int}
// expect: 53:49: error: type mismatch: found Container{type Elem <: String}, required Container{type Elem = String}
// expect: 7 errors found
trait Container:
  type Elem
  type F[_]
  def first: Elem
  def wrap(e: Elem): F[Elem]
class Names extends Container:
  type Elem = String
  type F[X] = List[X]
  def first: Elem = "n"
  def wrap(e: Elem): F[Elem] = List(e)
type Fixed[E] = Container { type Elem = E }

@main def run(): Unit =
  val a: Container = Names()
  val b: Container = Names()
  val same: a.Elem = a.first
  val alias: a.type = a
  val viaAlias: alias.Elem = a.first
  val backAgain: a.Elem = alias.first
  val proj: Container#Elem = a.first
  val names = Names()
  val str: String = names.first
  val asProj: Names#Elem = names.first
  val fixed: Fixed[String] = names
  val fromFixed: String = fixed.first
  val bounded: Container { type Elem <: AnyRef } = names
  val ref: AnyRef = bounded.first
  val eq: a.Elem =:= a.Elem = summon[a.Elem =:= a.Elem]
  val wrong: a.Elem = b.first
  val wrongProj: Container#Elem = b.first
  def any: Container = a
  val notPath: a.Elem = any.first
  val fa: a.F[Int] = ???
  val fb: b.F[Int] = ???
  val notInt: Int = a.first
  val okF: a.F[Int] = fa
  val wrongArg: a.F[String] = fa
  val wrongPrefix: a.F[Int] = fb
  val fixedOk: Container { type Elem = String } = names
  val fixedWrong: Container { type Elem = Int } = names
  val loose: Container { type Elem >: Nothing <: String } = fixedOk
  val stillLoose: Container { type Elem <: AnyRef } = loose
  val tight: Container { type Elem = String } = loose
  println(str + ref + fromFixed)
