package ui

trait Sized:
  def size: Int
  def sized: String = s"size $size"

enum Flavor extends Shape:
  case Sweet, Sour
  def name: String = toString.toLowerCase
  def size: Int = ordinal + 10

class Tag(val t: String) extends Shape:
  def name: String = t + Tone.Loud.size

object RightConst:
  val base: Int = Tone.Quiet.size + 100

val rightTop: String = "rightTop " + LeftConst.text
