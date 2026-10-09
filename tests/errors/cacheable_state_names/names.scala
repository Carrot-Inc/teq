// teq: --cacheable-state names.Nowhere --cacheable-state names.Nowhere --cacheable-state names.Outer.Inner --cacheable-state fix.runtime.Tokens
// jars: fixtures
// expect: error: --cacheable-state names.Nowhere: no such object
// expect: error: --cacheable-state: names.Plain is a class; cacheable state names an object
// expect: error: --cacheable-state: names.Shape is a trait; cacheable state names an object
// expect: error: --cacheable-state: names.Holder.Inner is an object nested in a class, which has an instance per enclosing instance
// expect: error: --cacheable-state: names.Host.Inner is an object nested in a trait, which has an instance per enclosing instance
// expect: 5 errors found
// The names of the flags and of the teq.toml together, one named twice: an object nested in
// objects and a jar's object are accepted; nothing, a class, a trait and an object nested in a
// class or a trait are errors of the build, each saying what the name found.
package names

class Plain
trait Shape
class Holder:
  object Inner
trait Host:
  object Inner
object Outer:
  object Inner

@main def run(): Unit = println(Outer.Inner)
