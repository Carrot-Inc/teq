// jars: scala-library scalajs-react
// targets: js
// scalajs-react 4.0.0 from its jars: `Callback`, `CallbackTo`, `CallbackOption`, `AsyncCallback`
// and the cats and cats-effect interop (`SyncIO` both ways, `IO` into `AsyncCallback` and back), run under node.
// Runs since these came in: first-wins classes, explicit using clauses of jar givens, `js.Array`'s
// mutations and cats-effect's JavaScript platform in the std. Its output needs no React.
// The expectation is scalac 3.8.4's: `scala-cli --power package <file> -o out.mjs` with the directives below (the
// application's five artifacts, in its order), run by node from a directory whose parent chain has `react` and
// `react-dom` 19 in a node_modules.
//> using scala 3.8.4
//> using platform scala-js
//> using jsModuleKind es
//> using dep com.github.japgolly.scalajs-react::callback::4.0.0
//> using dep com.github.japgolly.scalajs-react::callback-ext-cats::4.0.0
//> using dep com.github.japgolly.scalajs-react::callback-ext-cats_effect::4.0.0
//> using dep com.github.japgolly.scalajs-react::core-bundle-cats_effect::4.0.0
//> using dep com.github.japgolly.scalajs-react::extra::4.0.0
package sjrcallback

import cats.effect.{IO, SyncIO}
import cats.syntax.all.*
import japgolly.scalajs.react.callback.*
import japgolly.scalajs.react.callback.CallbackCats.*
import japgolly.scalajs.react.callback.CallbackCatsEffect.*

@main def main(): Unit =
  val hello: Callback = Callback(println("hello"))
  val twenty: CallbackTo[Int] = CallbackTo(20).map(_ + 1)
  println((hello >> twenty).runNow())
  Callback.traverse(List(1, 2, 3))(i => Callback(println(s"item $i"))).runNow()
  Callback(println("skipped")).when_(false).runNow()
  Callback(println("taken")).when_(true).runNow()
  println(CallbackOption.option(Some(3)).map(_ * 2).asCallback.runNow())
  println(CallbackOption.require(false).asCallback.runNow())
  val pair: CallbackTo[(Int, String)] = (twenty, CallbackTo("x")).tupled
  println(pair.runNow())
  val viaSyncIO: SyncIO[Int] = callbackToSyncIO(twenty)
  println(viaSyncIO.unsafeRunSync())
  println(syncIOToCallback(SyncIO(7)).runNow())
  val attempt = CallbackTo[Int](throw new RuntimeException("boom")).attempt.runNow()
  println(attempt.left.map(_.getMessage))
  val async: AsyncCallback[Int] = AsyncCallback.pure(5).map(_ * 2)
  val fromIO: AsyncCallback[Int] = ioToAsyncCallback(IO.pure(1).map(_ + 100))
  val program =
    for
      a <- async
      _ <- AsyncCallback.delay(println(s"async $a"))
      b <- fromIO
      _ <- AsyncCallback.delay(println(s"from io $b"))
      c <- ioToAsyncCallback(asyncCallbackToIO(AsyncCallback.pure(3)).map(_ * 3))
      _ <- AsyncCallback.delay(println(s"round trip $c"))
    yield ()
  program.runNow()
  println("sync end")
