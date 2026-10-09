package use

import izumi.reflect.Tag
import infra.Aid

object Main:
  def main(args: Array[String]): Unit =
    println(Tag[Aid].closestClass)
    println(summon[scala.reflect.ClassTag[Aid]])
    val arr = Array[Aid](Aid.authorize(3))
    println(arr.getClass.getSimpleName)
    println(arr(0).value)
