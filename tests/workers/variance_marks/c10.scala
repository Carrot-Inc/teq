package p

import scala.annotation.unchecked.uncheckedVariance

class Box10[-A]:
  def last: Option[A @uncheckedVariance] = None
  def all: List[A @uncheckedVariance] = Nil

object Use10:
  def size: Int = Box11[Int]().all.size + Box11[String]().last.size
