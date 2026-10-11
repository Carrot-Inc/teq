// The `$init$` calls of a class's constructor, in scalac's order (dotty's `Mixin.superCallOpt`): one per trait the class
// mixes in anew but a `NoInits` one, a trait of lazy vals alone among them, none for a trait its superclass mixes in.
// tests/run_jvm.sh lists them from javap's code of the constructors of `inits.LazyMix`, `inits.Sub` and `inits.Obj$`.
package inits
trait LazyOnly { lazy val x: String = "x" }
trait WithVal { val y: Int = 1 }
trait DefsOnly { def z: Int = 2 }
trait Stmt { println("stmt") }
class LazyMix extends LazyOnly with WithVal with DefsOnly
class Sub extends LazyMix with LazyOnly with Stmt
object Obj extends LazyOnly with DefsOnly
