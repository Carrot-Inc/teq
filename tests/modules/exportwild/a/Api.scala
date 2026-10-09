package ewa

class IA[A]:
  def a(x: A): A = x
  def shared: String = "a"
object A extends IA[String]

class IB[B]:
  def b(x: B): B = x
  def other: Int = 2
object B extends IB[Int]

object C:
  def c: Int = 3
  def hidden: Int = 4

object Api:
  export A.*
  export B.*
  export C.{hidden as _, *}
