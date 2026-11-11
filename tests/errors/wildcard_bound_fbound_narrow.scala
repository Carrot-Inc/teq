// `narrowBound` (TypeOps.scala 636-684) only where it is dotty's: the argument a parameter's own F-bound names
// (`lazyRefs`, the `LazyRef` of `BS`'s `S <: BS[T, S]`, Checking.scala 386) and its bound's wildcards massaged. The
// GADT lower bound it installs, `BS[BS[?, S]#T, S]`, holds the check only where the checked bound is that one
// (pi-scratch/r23b/dotty/probe.log). scalac 3.8.4 rejects each line:
//   23:48 Type argument S does not conform to upper bound BS[BS[? <: String, S]#T, S]
//   24:46 Type argument R does not conform to upper bound BS[BS[?, R]#T, R]
//   25:38 Type argument S does not conform to upper bound BS[BS[?, S]#T, S]
//   26:38 Type argument S does not conform to upper bound BS[BS[? <: Int, S]#T, S]
//   27:18 Type argument S does not conform to upper bound BS[BS[?, S]#T, S]
//   27:35 Type argument S does not conform to upper bound BS[BS[?, S]#T, S]
//   28:18 Type argument S does not conform to upper bound BS[BS[?, S]#T, S]
//   28:35 Type argument S does not conform to upper bound BS[BS[?, S]#T, S]
//   29:32 Type argument S does not conform to upper bound BS[BS[?, S]#T, S]
//   30:39 Type argument S does not conform to upper bound B3[B3[?, ?, S]#T, B3[?, ?, S]#U, S]
//   31:18 Type argument S does not conform to upper bound NB[NB[?, S]#T, ?]
//   31:31 Type argument S does not conform to upper bound NB[NB[?, S]#T, ?]
//   34:15 Type argument H.this.S does not conform to upper bound BS[BS[?, H.this.S]#T, H.this.S]
trait BS[T, S <: BS[T, S]]
trait BS2[T, S <: BS2[T, S]] extends BS[T, S]
trait B3[T, U, S <: B3[T, U, S]]
trait NB[T, S2 <: NB[T, ?]]
trait K
def g1[S <: BS[? <: Int, S]](x: BS[? <: String, S]): Int = 1
def g2[S <: BS[?, S], R <: BS[?, S]](x: BS[?, R]): Int = 1
def g3[S <: BS[? <: Int, S]](x: BS[?, S]): Int = 1
def g4[S <: BS[?, S]](x: BS[? <: Int, S]): Int = 1
def g5[S <: BS[?, S] & K](x: BS[?, S]): Int = 1
def g6[S <: BS[?, S] | K](x: BS[?, S]): Int = 1
def g7[S <: BS2[?, S]](x: BS[?, S]): Int = 1
def g8[S <: B3[?, Int, S]](x: B3[?, ?, S]): Int = 1
def g9[S <: NB[?, S]](x: NB[?, S]): Int = 1
trait H:
  type S <: BS[? <: Int, S]
  def x: BS[?, S]
// expect: 23:49: error: type argument S does not conform to upper bound BS[BS[? <: String, S]#T, S]
// expect: 24:47: error: type argument R does not conform to upper bound BS[BS[?, R]#T, R]
// expect: 25:39: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 26:39: error: type argument S does not conform to upper bound BS[BS[? <: Int, S]#T, S]
// expect: 27:19: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 27:36: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 28:19: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 28:36: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 29:33: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 30:40: error: type argument S does not conform to upper bound B3[B3[?, ?, S]#T, B3[?, ?, S]#U, S]
// expect: 31:19: error: type argument S does not conform to upper bound NB[NB[?, S]#T, ?]
// expect: 31:32: error: type argument S does not conform to upper bound NB[NB[?, S]#T, ?]
// expect: 34:16: error: type argument H.this.S does not conform to upper bound BS[BS[?, H.this.S]#T, H.this.S]
// expect: 13 errors found
