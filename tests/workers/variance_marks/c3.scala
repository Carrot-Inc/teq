package p

import scala.annotation.unchecked.uncheckedVariance

class Box3[-A]:
  def last: Option[A @uncheckedVariance] = None
  def all: List[A @uncheckedVariance] = Nil

object Use3:
  def size: Int = Box4[Int]().all.size + Box4[String]().last.size
