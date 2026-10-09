package p

import scala.annotation.unchecked.uncheckedVariance

class Box11[-A]:
  def last: Option[A @uncheckedVariance] = None
  def all: List[A @uncheckedVariance] = Nil

object Use11:
  def size: Int = Box0[Int]().all.size + Box0[String]().last.size
