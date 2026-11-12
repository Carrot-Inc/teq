// A higher-kinded type parameter applied is bounds-checked against its own parameters' bounds: `F[_ <: Int]`'s
// `typeParams` are its bound's lambda parameters (`checkAppliedType`'s `LambdaParam` instantiate, Checking.scala
// 116-123), so `F[? >: String]` needs `String <: Int`, and `F[String]` too. scalac 3.8.4 rejects both (E057):
//   8:24 Type argument  >: String does not overlap with upper bound Int
//   9:24 Type argument String does not conform to upper bound Int
// and accepts `F[Int]`. The expectation shows the wildcard as `? >: String`. (Collecting no parameters
// for the application of a type parameter accepts all three.)
def f[F[_ <: Int]](x: F[? >: String]): Int = 1
def g[F[_ <: Int]](x: F[String]): Int = 1
def h[F[_ <: Int]](y: F[Int]): Int = 1
// expect: 8:25: error: type argument ? >: String does not overlap with upper bound Int
// expect: 9:25: error: type argument String does not conform to upper bound Int
// expect: 2 errors found
