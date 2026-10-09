package memo

import Instances.given

object Use:
  def int: String = summon[TC[Int]].name
  def str: String = summon[TC[String]].name
  def key: String = summon[TC[Key]].name
  def local: String =
    summon[TC[Int]].name
