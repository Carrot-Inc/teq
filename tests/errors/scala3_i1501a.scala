// Adapted from scala3 tests/neg/i1501a.scala and tests/neg/templateParents.scala (Apache-2.0, see tests/scala3/README.md); replaced: the classes nested in objects stand at the top level.
// expect: 10:21: error: trait TSubA may not call constructor of class SubA
// expect: 11:7: error: missing argument for parameter x
// expect: 15:17: error: trait D may not call constructor of class C
// expect: 16:28: error: class C2 is not a trait
// expect: 4 errors found
class A
class SubA(x: Int) extends A
trait TA extends A
trait TSubA extends SubA(2)
class Foo extends TA with TSubA

class C(x: String)
class C2
trait D extends C("a")
val made = new C("b") with C2
