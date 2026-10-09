package p

import scala.annotation.unchecked.uncheckedVariance

class Box1[-A]:
  def last: Option[A @uncheckedVariance] = None
  def all: List[A @uncheckedVariance] = Nil

object Use1:
  def size: Int = Box2[Int]().all.size + Box2[String]().last.size
