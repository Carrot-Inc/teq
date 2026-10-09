// jars: scala-library cats-kernel-sjs cats-core-sjs
// cats.data from the jars: NonEmptyList, NonEmptySet, NonEmptyMap, NonEmptyChain, Validated and
// ValidatedNec with mapN, andThen and toEither, Ior, Chain, EitherT and OptionT over Option,
// State and Kleisli.
//> using dep org.typelevel::cats-core:2.13.0
import cats.data.*
import cats.syntax.all.*
import cats.{Eval, Semigroup, Show}

object Main:
  def p(xs: Any*): Unit = println(xs.mkString(" "))
  def validName(s: String): ValidatedNec[String, String] =
    if s.nonEmpty then s.validNec else "empty name".invalidNec
  def validAge(n: Int): ValidatedNec[String, Int] =
    if n >= 0 then n.validNec else s"negative age $n".invalidNec
  case class Person(name: String, age: Int)

  def main(args: Array[String]): Unit =
    val nel = NonEmptyList.of(3, 1, 2)
    println(nel)
    p(nel.head, nel.tail, nel.size, nel.last)
    p(nel.map(_ * 2), nel.sorted, nel.reverse, nel.toList)
    p(nel.reduceLeft(_ + _), nel.foldLeft(0)(_ + _), nel.exists(_ > 2), nel.filter(_ > 1))
    println(nel ++ List(4, 5))
    p(nel.concatNel(NonEmptyList.one(9)), nel.append(7), nel.prepend(0))
    p(nel.zipWithIndex, nel.groupBy(_ % 2), nel.distinct, nel.sortBy(-_))
    p(NonEmptyList.fromList(List.empty[Int]), NonEmptyList.fromList(List(1)), List(1, 2).toNel)
    p(nel.traverse(i => Option(i).filter(_ > 0)), nel.mkString_("[", ";", "]"), nel.show)
    p(nel.collect { case i if i > 1 => i * 10 }, nel.find(_ == 1), nel.exists(_ == 2))
    p((nel, NonEmptyList.of("a", "b", "c")).mapN(_.toString + _), nel.zip(NonEmptyList.of("x", "y", "z")))
    p(NonEmptyList.of(("a", 1), ("b", 2)).toNem, nel.toNes)
    val nes = NonEmptySet.of(3, 1, 2)
    p(nes, nes.head, nes.contains(2), nes.add(0), nes.toSortedSet, nes.length)
    p(nes.map(_ * 2), nes.filter(_ > 1), (nes ++ NonEmptySet.of(9, 1)), nes.toNonEmptyList)
    p(NonEmptySet.fromSet(scala.collection.immutable.SortedSet(5, 4)), NonEmptySet.fromSet(scala.collection.immutable.SortedSet.empty[Int]))
    val nem = NonEmptyMap.of("b" -> 2, "a" -> 1)
    p(nem, nem.head, nem.lookup("a"), nem.lookup("z"), nem.keys, nem.toSortedMap)
    p(nem.add("c" -> 3), nem.map(_ * 10), nem.length, nem.contains("b"), nem.toNel)
    p(nem.updateWith("a")(_ + 5), nem.mapBoth((k, v) => (k + k, v + 1)), nem.foldLeft(0)(_ + _))
    val nec = NonEmptyChain.of(1, 2, 3)
    p(nec, nec.head, nec.tail, nec.length, nec.toNonEmptyList, nec.toChain)
    p(nec.map(_ + 1), nec.append(4), nec.prepend(0), (nec ++ NonEmptyChain.one(9)), nec.reverse)
    p(nec.toList, nec.filter(_ % 2 == 1), nec.exists(_ > 2), nec.reduceLeft(_ + _), nec.distinct)
    p(NonEmptyChain.fromSeq(List(1)), NonEmptyChain.fromSeq(Nil), NonEmptyChain.fromNonEmptyList(nel), nec.show)
    val ok = (validName("ann"), validAge(3)).mapN(Person.apply)
    val bad = (validName(""), validAge(-1)).mapN(Person.apply)
    p(ok, bad)
    p(ok.toEither, bad.toEither.leftMap(_.toList), ok.isValid, bad.isInvalid)
    p(ok.map(_.age), bad.leftMap(_.length), ok.getOrElse(Person("", 0)), bad.toOption)
    p(validAge(1).andThen(a => validAge(a - 5)), validAge(1).andThen(a => validAge(a + 5)))
    p(bad.fold(_.toList.mkString("|"), _.name), ok.fold(_.toList.mkString("|"), _.name))
    p(Validated.fromEither(Left("e"): Either[String, Int]), Validated.fromOption(Option(2), "none"), Validated.cond(false, 1, "no"))
    p(Validated.catchNonFatal(throw new IllegalStateException("x")).leftMap(_.getMessage), 1.valid[String].toValidatedNec)
    println((1.validNel[String], "e".invalidNel[Int], "f".invalidNel[Int]).tupled)
    p(("a".invalid[Int] combine "b".invalid[Int]), (1.valid[String] combine 2.valid[String]), Validated.valid[String, Int](1).orElse(2.valid))
    p(bad.swap, ok.bimap(_.length, _.name), bad.forall(_.age > 0), ok.exists(_.age > 0))
    p(List(validAge(1), validAge(2), validAge(-3)).sequence, List(validAge(1), validAge(2)).sequence)
    p(List(1, -2, -3).traverse(validAge), List(1, 2).traverse(validAge).map(_.sum))
    val ior1: Ior[String, Int] = Ior.both("warn", 1)
    p(ior1, Ior.left[String, Int]("l"), Ior.right[String, Int](2), ior1.map(_ + 1), ior1.left, ior1.right)
    p(ior1.fold(_ => 0, identity, (_, b) => b * 10), ior1.isBoth, ior1.toEither, ior1.toValidated, ior1.pad)
    p((Ior.both("a", 1) combine Ior.both("b", 2)), Ior.both("a", 1).flatMap(i => Ior.right(i + 1)), Ior.fromOptions(Option("x"), Option.empty[Int]))
    val chain = Chain(1, 2) ++ Chain.one(3) :+ 4
    p(chain, chain.toList, chain.length, chain.headOption, chain.lastOption, chain.isEmpty)
    p(chain.map(_ * 2), chain.filter(_ > 2), chain.reverse, (0 +: chain), chain.uncons, chain.initLast)
    p(Chain.fromSeq(List(1, 2)), Chain.empty[Int], Chain.fromOption(Option(1)), chain.foldLeft(0)(_ + _), chain.toVector)
    p(chain.zipWithIndex, chain.groupBy(_ % 2), chain.contains(3), chain.exists(_ > 3), chain.get(1), chain.take(2))
    p(chain.show, (chain === Chain(1, 2, 3, 4)), chain.distinct, chain.sortBy(-_), chain.deleteFirst(_ > 1))
    val et: EitherT[Option, String, Int] = EitherT(Option(Right(1): Either[String, Int]))
    p(et.value, et.map(_ + 1).value, et.flatMap(i => EitherT.rightT[Option, String](i * 5)).value, et.leftMap(_.length).value)
    p(EitherT.leftT[Option, Int]("bad").value, EitherT.fromOption[Option](Option.empty[Int], "none").value, EitherT.fromEither[Option](Right(2): Either[String, Int]).value)
    p(et.isRight, et.getOrElse(0), et.fold(_.length, _ + 100), et.toOption.value, EitherT.liftF[Option, String, Int](Option(7)).value)
    p((for a <- et; b <- EitherT.rightT[Option, String](10) yield a + b).value, et.semiflatMap(i => Option(i.toString)).value, et.subflatMap(i => Left("s"): Either[String, Int]).value)
    val ot: OptionT[List, Int] = OptionT(List(Option(1), None, Option(3)))
    p(ot.value, ot.map(_ * 2).value, ot.getOrElse(0), ot.isDefined, ot.fold(0)(_ + 1))
    p(OptionT.pure[List](5).value, OptionT.none[List, Int].value, OptionT.liftF(List(1, 2)).value, OptionT.fromOption[List](Option(9)).value)
    p(ot.flatMap(i => OptionT(List(Option(i + 1)))).value, ot.filter(_ > 1).value, ot.orElse(OptionT.pure[List](0)).value, ot.toRight("l").value)
    val counter: State[Int, String] = for
      a <- State.get[Int]
      _ <- State.set(a + 10)
      _ <- State.modify[Int](_ * 2)
      b <- State.inspect[Int, Int](_ + 1)
    yield s"$a->$b"
    p(counter.run(1).value, counter.runA(2).value, counter.runS(3).value)
    p(State.pure[Int, String]("p").run(0).value, List(1, 2, 3).traverse(i => State[Int, Int](s => (s + i, s * i))).run(1).value)
    val k1: Kleisli[Option, Int, Int] = Kleisli(i => if i > 0 then Some(i * 2) else None)
    val k2: Kleisli[Option, Int, String] = Kleisli(i => Some(i.toString))
    p(k1.run(2), k1.run(-1), (k1 andThen k2).run(3), k1.map(_ + 1).run(1), k1.flatMap(i => Kleisli((j: Int) => Option(i + j))).run(4))
    p(Kleisli.pure[Option, Int, String]("c").run(0), Kleisli.ask[Option, Int].run(8), k1.local[Int](_ + 1).run(1), (k1, k1).mapN(_ + _).run(5))
    p(Eval.now(1).map(_ + 1).value, Eval.later(2).flatMap(x => Eval.now(x * 3)).value, Eval.defer(Eval.always(4)).value, Eval.now(5).memoize.value)
    p(Writer("log", 1).run, Writer("a", 1).flatMap(i => Writer("b", i + 1)).run, Writer.value[String, Int](3).written, Writer.tell("t").value)
    p(Const[String, Int]("c").getConst, Const("a").map(_ => 1).getConst, (Const[String, Int]("x") combine Const[String, Int]("y")).getConst)
    p(Nested(Option(List(1, 2))).map(_ + 1).value, Tuple2K(Option(1), List(2)).first, Func.func((i: Int) => Option(i + 1)).run(1))
    p(NonEmptyVector.of(1, 2).append(3), NonEmptyVector.fromVector(Vector.empty[Int]), NonEmptyVector.of(1, 2).toVector, NonEmptyVector.of(1, 2).head)
    p(NonEmptyList.of(1, 2, 3).foldRight(Eval.now(0))((a, b) => b.map(_ + a)).value, NonEmptyList.of(1, 2).flatMap(i => NonEmptyList.of(i, i)), (NonEmptyList.of(1) ::: NonEmptyList.of(2)))
    p(Semigroup[NonEmptyChain[Int]].combine(nec, nec).length, Show[NonEmptyList[Int]].show(nel), summon[Show[Validated[String, Int]]].show(1.valid))
