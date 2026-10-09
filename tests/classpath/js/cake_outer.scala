// jars: fixtures
// targets: js interp jvm
// A trait nested in a class of a jar reaches the enclosing instance through the accessor the
// classes mixing it in implement, as scalac's outer accessor does: `Module.this.Named` inside
// `HelperModule`, an inner class made there, an object two deep reading the outer's outer, and
// an object implementing an abstract val read through the parent only. A self type's members in
// the signatures of an inner trait's members are the receiver's (`SelfOther.h`, `viaParam`), and
// an object of a platform trait implements a module trait's member over the self type's alias.
import fix.cake.{ModuleImpl, ModulePlatformImpl, PromisesImpl, SelfImpl}

@main def main(): Unit =
  val m = new ModuleImpl("!")
  println(m.useIt("a"))
  println(m.inner)
  println(new ModulePlatformImpl().useIt("b"))
  val s = new SelfImpl
  println(s.g(List(1)))
  println(s.h(List(2)))
  println(s.viaParam(s)(List(3)))
  println(new PromisesImpl().run("n"))
