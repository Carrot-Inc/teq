// `import JSConverters.*` brings the extensions; `import JSConverters.JSRichOption` (the names of the original's
// implicit classes) brings a given whose object holds the same extension, which the compiler searches as well.
package scala.scalajs.js

object JSConverters:
  extension [A](self: Option[A])
    def orUndefined: UndefOr[A] = self match
      case Some(a) => a
      case None => undefined

  extension [A](self: IterableOnce[A])
    @js("Array.from($0)")
    def toJSArray: Array[A]
    @js("Array.from($0)")
    def toJSIterable: Iterable[A]

  extension [A](self: Map[String, A])
    def toJSDictionary: Dictionary[A] = Dictionary(self.toList*)

  extension [A](self: scala.concurrent.Future[A])
    def toJSPromise(implicit ec: scala.concurrent.ExecutionContext): Promise[A] = JSRichFutureOps.toJSPromise(self)

  object JSRichOptionOps:
    extension [A](self: Option[A])
      def orUndefined: UndefOr[A] = self match
        case Some(a) => a
        case None => undefined

  object JSRichIterableOnceOps:
    extension [A](self: IterableOnce[A])
      @js("Array.from($0)")
      def toJSArray: Array[A]
      @js("Array.from($0)")
      def toJSIterable: Iterable[A]

  object JSRichMapOps:
    extension [A](self: Map[String, A])
      def toJSDictionary: Dictionary[A] = Dictionary(self.toList*)

  // Scala.js's `JSRichFutureNonThenable`: a promise the future completes, its failure's value unwrapped.
  object JSRichFutureOps:
    extension [A](self: scala.concurrent.Future[A])
      def toJSPromise(implicit ec: scala.concurrent.ExecutionContext): Promise[A] =
        new Promise[A]((resolve: Function1[A | Thenable[A], scala.Any], reject: Function1[scala.Any, scala.Any]) =>
          self.onComplete {
            case scala.util.Success(value) => resolve(value)
            case scala.util.Failure(th) => reject(special.unwrapFromThrowable(th))
          })

  given JSRichOption: JSRichOptionOps.type = JSRichOptionOps
  given JSRichFutureNonThenable: JSRichFutureOps.type = JSRichFutureOps
  given JSRichIterableOnce: JSRichIterableOnceOps.type = JSRichIterableOnceOps
  given JSRichIterable: JSRichIterableOnceOps.type = JSRichIterableOnceOps
  given JSRichMap: JSRichMapOps.type = JSRichMapOps
