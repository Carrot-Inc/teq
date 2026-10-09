package p

import scala.annotation.unchecked.uncheckedVariance

class Box8[-A]:
  def last: Option[A @uncheckedVariance] = None
  def all: List[A @uncheckedVariance] = Nil

object Use8:
  def size: Int = Box9[Int]().all.size + Box9[String]().last.size
