// An implicit object nested in a trait is the given of its own singleton type through the object
// that extends the trait: `UseDefault.type` of the trait's `check` is `Assertions.UseDefault.type`
// at a call through the object, and the object's inherited `UseDefault` is that reference (dotty's
// `TermRef(Assertions, UseDefault)`, its module class conforming to it, `TypeComparer.compareNamed`),
// whether the object is imported, written in full, or the call stands inside the trait or the object,
// for a plain and an inline method.
package lib:
  trait Assertions:
    implicit object UseDefault:
      def name = "default"
    def check(implicit use: UseDefault.type): String = "ok " + use.name
    inline def icheck(inline x: Boolean)(implicit use: UseDefault.type): String = "ok " + x + " " + use.name
    def inside: String = check + " / " + icheck(2 > 1)
  object Assertions extends Assertions:
    def own: String = check + " / " + icheck(true)

object Imported:
  import lib.Assertions.*
  def show(): Unit =
    println(check)
    println(icheck(1 == 1))
    println(summon[UseDefault.type] eq UseDefault)

@main def run(): Unit =
  Imported.show()
  println(lib.Assertions.check)
  println(lib.Assertions.icheck(1 == 2))
  println(lib.Assertions.inside)
  println(lib.Assertions.own)
  println(summon[lib.Assertions.UseDefault.type] eq lib.Assertions.UseDefault)
