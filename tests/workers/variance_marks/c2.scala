package p

import scala.annotation.unchecked.uncheckedVariance

class Box2[-A]:
  def last: Option[A @uncheckedVariance] = None
  def all: List[A @uncheckedVariance] = Nil

object Use2:
  def size: Int = Box3[Int]().all.size + Box3[String]().last.size
