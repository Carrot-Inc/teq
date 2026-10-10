// A plain inline method's call whose result is inferred has the type the definition inferred
// (`Namer.inferredResultType`), instantiated at the call, not its expansion's: `label[1]` is a
// `ToString[1]`, which is `"1"`, and `size[List[Int]]` a `Size[List[Int]]`, which is `3`.
import scala.compiletime.constValue
import scala.compiletime.ops.int.{ToString, +}
type Size[T] <: Int = T match
  case List[t] => 3
  case _ => 0
inline def label[N <: Int] = constValue[ToString[N]]
inline def next[N <: Int] = constValue[N + 1]
inline def size[T] = constValue[Size[T]]
@main def run(): Unit =
  val a: "1" = label[1]
  val b: 3 = next[2]
  val c: 3 = size[List[Int]]
  val d = label[42]
  println(s"$a $b $c ${d.length}")
