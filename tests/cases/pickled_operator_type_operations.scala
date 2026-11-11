import scala.compiletime.{constValue, ops}

object Main:
  type Named = ops.int.+[6, 1]
  type Chosen[N <: Int] <: Int = N match
    case 6 => Named
  inline def selected[N <: Int] = constValue[Chosen[N]]
  inline def count[N <: Int] = constValue[ops.int.+[N, 1]]
  inline def flip[N <: Boolean] = constValue[ops.boolean.![N]]
  inline def label[N <: Int] = constValue[ops.any.ToString[N]]

  def main(args: Array[String]): Unit =
    println(count[6] + 1)
    println(flip[false] & true)
    println(label[7] + label[8])
    println(selected[6] + 1)
