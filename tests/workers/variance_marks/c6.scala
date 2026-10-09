package p

import scala.annotation.unchecked.uncheckedVariance

class Box6[-A]:
  def last: Option[A @uncheckedVariance] = None
  def all: List[A @uncheckedVariance] = Nil

object Use6:
  def size: Int = Box7[Int]().all.size + Box7[String]().last.size
