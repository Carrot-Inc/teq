// An inline expansion's type avoids its bindings (`TypeAssigner.assignType` of an `Inlined`,
// `avoidingType`): a result naming a parameter whose argument is no path is, outside the
// expansion, the argument's type, `Option[Int]`, never the singleton of the parameter's proxy;
// a transparent expansion's as a plain one's.
extension (o: Option[Int])
  inline def self: o.type = o
  transparent inline def same = o.self
def f(): Unit =
  val s = Option(7).self
  val t: Int = s
  val u: Int = Option(8).same
  ()
// expect: 10:16: error: Found:    Option[Int]
// expect+ Required: Int
// expect: 11:16: error: Found:    Option[Int]
// expect+ Required: Int
// absent: (o : Option[Int])
