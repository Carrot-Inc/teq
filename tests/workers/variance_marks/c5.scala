package p

import scala.annotation.unchecked.uncheckedVariance

class Box5[-A]:
  def last: Option[A @uncheckedVariance] = None
  def all: List[A @uncheckedVariance] = Nil

object Use5:
  def size: Int = Box6[Int]().all.size + Box6[String]().last.size
