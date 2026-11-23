// `Object`'s `toString` of a source package object's object and of an instance of its class: the runtime class's
// name, `p.package$O$` and `p.package$K` (dotty's `Desugar.packageModuleDef`), and a hash code, cut here, which
// differs from run to run.
package object p:
  object O
  class K

@main def run(): Unit =
  println(p.O.toString.takeWhile(_ != '@'))
  println((new p.K).toString.takeWhile(_ != '@'))
