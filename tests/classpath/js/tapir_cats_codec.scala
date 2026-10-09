// jars: scala-library tapir-core tapir-cats cats-kernel-sjs cats-core-sjs sttp-model sttp-shared-core sttp-shared-ws magnolia123 scala-java-time
//> using dep com.softwaremill.sttp.tapir::tapir-core:1.13.29
//> using dep com.softwaremill.sttp.tapir::tapir-cats:1.13.29
// tapir 1.13.29's cats integration from its jar (`sttp.tapir.integ.cats.codec.*`): the schemas
// and codecs of `NonEmptyList`, `NonEmptySet`, `NonEmptyChain`, `NonEmptyVector` and `Chain`, a
// query of a `NonEmptyList`, and the non-empty validators over cats' `Foldable`.
package tapircats

import cats.data.{Chain, NonEmptyChain, NonEmptyList, NonEmptySet, NonEmptyVector}
import sttp.tapir.*
import sttp.tapir.integ.cats.codec.*

object Main:
  def main(args: Array[String]): Unit =
    val nel = summon[Schema[NonEmptyList[Int]]]
    println(nel.schemaType.toString + " " + nel.validator.show + " " + nel.isOptional)
    println(nel.applyValidation(NonEmptyList.of(1, 2)).toString)
    val nes = summon[Schema[NonEmptySet[String]]]
    println(nes.schemaType.toString + " " + nes.validator.show)
    println(summon[Schema[NonEmptyChain[Int]]].validator.show.toString + " " + summon[Schema[NonEmptyVector[Int]]].validator.show + " " + summon[Schema[Chain[Int]]].schemaType)
    val q = query[NonEmptyList[Int]]("ids")
    println(q.show + " " + q.codec.schema.isOptional)
    println(q.codec.decode(List("1", "2", "3")).toString)
    println(q.codec.decode(Nil).toString)
    println(q.codec.decode(List("x")).getClass.getSimpleName)
    println(q.codec.encode(NonEmptyList.of(4, 5)))
    val s = query[NonEmptySet[Int]]("set")
    println(s.codec.decode(List("3", "1", "3")).toString + " " + s.codec.encode(NonEmptySet.of(2, 1)))
    val c = header[NonEmptyChain[String]]("X-Tags")
    println(c.codec.decode(List("a", "b")).toString + " " + c.codec.encode(NonEmptyChain.of("z")))
    val v = query[NonEmptyVector[Long]]("v")
    println(v.codec.decode(List("7")).toString + " " + v.codec.decode(Nil))
    val ch = query[Chain[Int]]("chain")
    println(ch.codec.decode(Nil).toString + " " + ch.codec.decode(List("1", "2")) + " " + ch.codec.encode(Chain(9)))
    val ep = endpoint.get.in("items" / query[NonEmptyList[Int]]("ids")).out(header[NonEmptyList[String]]("X-Names"))
    println(ep.show)
    println(nel.applyValidation(NonEmptyList.one(0)).length.toString + " " + nel.validator.toString.startsWith("Custom") )
