// Adapted from scala3 tests/neg/i4368.scala (Apache-2.0, see tests/scala3/README.md): two traits
// that define `A` and `B` through each other make a class mixing both cyclic.
// expect: 11:7: error: illegal cyclic type reference: alias Z.A of type A refers back to the type itself
// expect: 2 errors found
trait X:
  type A = B
  type B
trait Y:
  type A
  type B = A
trait Z extends X with Y
@main def run(): Unit = ()
