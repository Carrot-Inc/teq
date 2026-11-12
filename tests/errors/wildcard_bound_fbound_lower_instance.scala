// A lower-bounded wildcard of an F-bounded parameter, its lower end an instance of the F-bound: `D[? >: E]` for
// `E extends D[E]`. The bound `D[T]` is seen from the application with the wildcard's own parameter a skolem,
// `D[D[? >: E]#T]` (Checking.scala 118, Types.scala 2794), and `E <: D[<app>#T]` needs `E =:= <app>#T`, which
// `<app>#T <: E` fails (its upper end is `D[<app>#T]`). scalac 3.8.4 rejects all three (E057):
//   15:11 Type argument  >: E does not overlap with upper bound D[D[? >: E]#T]
//   17:11 Type argument  >: String does not overlap with upper bound Comparable[C[? >: String]#T]
//   18:24 Type argument  >: java.util.concurrent.TimeUnit does not overlap with upper bound Enum[Enum[? >: java.util.concurrent.TimeUnit]#E]
// teq is to reject the three as scalac does; substituted into its
// own F-bound the wildcard gives `D[? >: E]`, which `E` is below.
// The expectation shows the wildcard as `? >: E`. (With a lower end
// that is no instance, `D[? >: String]`, scalac rejects as well.)
import java.util.concurrent.TimeUnit
class D[T <: D[T]]
class E extends D[E]
type X = D[? >: E]
class C[T <: Comparable[T]]
type Y = C[? >: String]
type Z = java.lang.Enum[? >: TimeUnit]
// expect: 15:12: error: type argument ? >: E does not overlap with upper bound D[D[? >: E]#T]
// expect: 17:12: error: type argument ? >: String does not overlap with upper bound Comparable[C[? >: String]#T]
// expect: 18:25: error: type argument ? >: java.util.concurrent.TimeUnit does not overlap with upper bound Enum[Enum[? >: java.util.concurrent.TimeUnit]#E]
// expect: 3 errors found
