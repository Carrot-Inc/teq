package bta

class V(val n: Int)
object Api:
  def a1[T <: String](x: Array[T]): Int = x.length
  def a5[T <: V](x: Array[T]): Int = x(0).n
  def plain(x: Array[String]): Int = x.length
