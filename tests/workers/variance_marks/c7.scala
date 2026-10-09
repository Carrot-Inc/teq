package p

import scala.annotation.unchecked.uncheckedVariance

class Box7[-A]:
  def last: Option[A @uncheckedVariance] = None
  def all: List[A @uncheckedVariance] = Nil

object Use7:
  def size: Int = Box8[Int]().all.size + Box8[String]().last.size
