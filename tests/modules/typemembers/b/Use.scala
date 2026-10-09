package tmb

import tma.*

object IntCodec extends Codec:
  type T = Int
  type Wire = String
  def encode(t: Int): String = t.toString
  def decode(w: String): Int = w.toInt

class MapRegistry extends Registry:
  type Value = Int
  private var entries: List[(String, Int)] = Nil
  def put(k: Key, v: Int): Unit = entries = (k, v) :: entries
  def size: Int = entries.size

object Use:
  def main(args: Array[String]): Unit =
    println(IntCodec.decode(IntCodec.encode(41)) + 1)
    val r = new MapRegistry
    r.put("a", 1)
    println(r.size)
