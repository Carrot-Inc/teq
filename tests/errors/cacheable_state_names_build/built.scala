// teq: --cacheable-state built.Nowhere --cacheable-state built.Nowhere --cacheable-state built.Outer.Inner --cacheable-state fix.runtime.Tokens
// jars: fixtures
// expect: error: --cacheable-state built.Nowhere: no such object
// expect: error: --cacheable-state: built.Plain is a class; cacheable state names an object
// expect: error: --cacheable-state: built.Shape is a trait; cacheable state names an object
// expect: error: --cacheable-state: built.Holder.Inner is an object nested in a class, which has an instance per enclosing instance
// expect: error: --cacheable-state: built.Host.Inner is an object nested in a trait, which has an instance per enclosing instance
// expect: 5 errors found
// command: build
// The same names under build, which checks them as check does.
package built

class Plain
trait Shape
class Holder:
  object Inner
trait Host:
  object Inner
object Outer:
  object Inner

@main def run(): Unit = println(Outer.Inner)
