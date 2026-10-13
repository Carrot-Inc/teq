// expect: 8:26: error: covariant type X occurs in contravariant position in type  <: X of type Y
// expect: 9:26: error: contravariant type X occurs in covariant position in type  >: X of type Y
// expect: 10:26: error: contravariant type X occurs in covariant position in type  >: X of type Y
// expect: 11:26: error: covariant type X occurs in contravariant position in type  <: X of type Y
// expect: 4 errors found
// Adapted from scala3 tests/neg/variances.scala (Apache-2.0, see tests/scala3/README.md): the
// trait cases without structural types.
trait Foo1[+X] { def bar[Y <: X](y: Y) = y }
trait Foo3[-X] { def bar[Y >: X](y: Y) = y }
trait Foo5[-X] { def bar[Y >: X](y: Y) = y }
trait Foo7[+X] { def bar[Y <: X](y: Y) = y }
trait Ok1[+X] { def bar[Y >: X](y: Y) = y }
trait Ok2[-X] { def bar[Y <: X](y: Y) = y }
@main def run(): Unit = println(1)
