import scala.collection.mutable

def show(label: String, values: Any*): Unit = println(label + ": " + values.mkString(" | "))

def sorted[V](m: Map[Int, V]): List[(Int, V)] = m.toList.sortBy(_._1)

@main def main(): Unit =
  val m = Map(1 -> "a", 2 -> "bb", 3 -> "ccc")
  show("basic", m(1), m.get(9), m.getOrElse(9, "z"), m.contains(2), m.size, m.isEmpty, m.nonEmpty, m.keys.toList.sorted, m.values.toList.sorted, m.keySet.toList.sorted, m.head, m.headOption, m.last)
  show("transform", sorted(m.map((k, v) => (k * 2, v.length))), sorted(m.filter((k, v) => k > 1)), sorted(m.filterNot(_._1 > 1)), sorted(m.view.mapValues(_.length).toMap), sorted(m.view.filterKeys(_ != 2).toMap), sorted(m.transform((k, v) => v * k)))
  show("fold", m.foldLeft(0)((acc, e) => acc + e._1), m.exists((k, v) => v == "bb"), m.forall(_._1 > 0), m.find(_._2.length == 3), m.count(_._1 % 2 == 1), m.toList.sortBy(_._1).map(_._2).mkString, m.maxBy(_._1), m.minBy(_._2.length), m.values.map(_.length).sum, m.keys.max)
  show("collect like", m.flatMap((k, v) => if k > 1 then Some(k -> v) else None).toList.sorted, m.toList.flatMap((k, v) => List.fill(k)(v)).sorted, m.partition(_._1 > 1)._1.size, m.groupBy(_._2.length % 2).toList.sortBy(_._1).map((k, v) => (k, sorted(v))), m.toSeq.sortBy(_._1).headOption, m.toVector.length, m.zipWithIndex.size)
  show("updates", sorted(m + (4 -> "d")), sorted(m ++ Map(1 -> "A", 5 -> "e")), sorted(m ++ List(6 -> "f")), sorted(m ++ Some(7 -> "g")), sorted(m - 1), sorted(m -- List(1, 2)), sorted(m.removed(3)), sorted(m.updatedWith(1)(_.map(_ + "!"))), sorted(m.concat(List(8 -> "h"))))
  show("defaults", m.getOrElse(1, "x"), m.applyOrElse(9, (k: Int) => "none"), m.isDefinedAt(1), m.keysIterator.toList.sorted, m.valuesIterator.toList.sorted, m.iterator.toList.sortBy(_._1).take(1), Map.empty[Int, Int].headOption)
  show("conversions", List(1 -> "a", 1 -> "b").toMap, List("a", "b").zipWithIndex.toMap.toList.sorted, List(1, 2, 3).map(x => x -> x * x).toMap.get(2), List("a", "bb").groupBy(_.length).view.mapValues(_.size).toMap.toList.sorted, Map(1 -> List(1, 2)).flatMap((k, v) => v.map(x => (x, k))).toList.sorted, m.toList.unzip._1.sorted, m.unzip._2.toList.sorted)
  val s = Set(3, 1, 2)
  show("set", s.map(_ * 2).toList.sorted, s.filter(_ > 1).toList.sorted, s.flatMap(x => Set(x, x + 10)).size, s.exists(_ > 2), s.forall(_ > 0), s.toList.sorted, s.size, s(1), s(9), s.head > 0, s.min, s.max, s.sum, s.isEmpty, (s + 4).size, (s - 1).size, (s ++ List(7, 8)).size, (s -- List(1, 2)).size, s.partition(_ > 1)._1.size, s.groupBy(_ % 2).size, s.toVector.sorted, s.mkString(",").length, s.foldLeft(0)(_ + _), s.find(_ == 2), s.count(_ > 1), s.zipWithIndex.size, s.map(_ % 2).size, s.contains(2), s.diff(Set(1)).toList.sorted, s.subsetOf(Set(1, 2, 3, 4)), s == Set(1, 2, 3), s.toSeq.sorted, s.headOption.isDefined, Set.empty[Int].headOption, s.intersect(Set(2, 9)), s.union(Set(9)).size, s.nonEmpty)
  val hm = mutable.HashMap(1 -> "a")
  hm(2) = "b"
  hm += (3 -> "c")
  hm.put(4, "d")
  hm -= 1
  hm.remove(2)
  hm.update(3, "C")
  show("mutable map", hm.toList.sortBy(_._1), hm.get(3), hm.getOrElseUpdate(9, "new"), hm.contains(9), hm.size, hm.keys.toList.sorted, hm.values.toList.sorted, hm.toMap.size, hm.map((k, v) => (k + 1, v)).toList.sortBy(_._1), hm.filter(_._1 > 3).size, hm.exists(_._2 == "C"), hm.getOrElse(100, "-"), hm.isEmpty, hm.foldLeft(0)(_ + _._1), hm.find(_._1 == 4), hm.keySet.size, hm.clone().size, hm.addOne(7 -> "x").size, hm.subtractOne(7).size, hm.updateWith(3)(_.map(_ + "!")), hm.filterInPlace((k, v) => k != 9).size, hm.mapValuesInPlace((k, v) => v.toLowerCase).toList.sortBy(_._1))
  val ms = mutable.Set(1, 2)
  ms += 3
  ms -= 1
  ms.add(4)
  ms.remove(2)
  ms ++= List(5, 6)
  ms --= List(6)
  show("mutable set", ms.toList.sorted, ms.contains(3), ms(9), ms.size, ms.toSet.size, ms.map(_ + 1).toList.sorted, ms.filter(_ > 3).toList.sorted, ms.add(3), ms.add(10), ms.remove(99), ms.isEmpty, ms.exists(_ == 10), ms.clone().size, ms.addOne(11).size, ms.subtractOne(11).size, ms.filterInPlace(_ < 10).size, ms.min, ms.sum)
  val buf = mutable.ArrayBuffer(1, 2, 3)
  buf += 4
  buf ++= List(5, 6)
  buf.prepend(0)
  buf.insert(1, 99)
  buf.remove(1)
  buf -= 6
  buf(0) = -1
  show("buffer", buf.toList, buf.length, buf.head, buf.last, buf.map(_ * 2).toList, buf.filter(_ > 2).toList, buf.indexOf(3), buf.contains(5), buf.sum, buf.sorted.toList, buf.reverse.toList, buf.take(2).toList, buf.isEmpty, buf.mkString(","), buf.toVector, buf.zipWithIndex.length, buf.foldLeft(0)(_ + _), buf.addOne(7).length, buf.append(8).length, buf.appendAll(List(9)).length, buf.prependAll(List(-3, -2)).length, buf.dropInPlace(1).toList, buf.clear(), buf.isEmpty, buf.result().length)
