package scala.runtime

// The part of the JVM runtime that names scala-library's collections, whose bytecode the output
// links against, the JVM's one mode.

def arrayRest(arr: Array[?], from: Int, until: Int): Seq[Any] =
  scala.collection.immutable.ArraySeq.unsafeWrapArray(arr).slice(from, until)

// `StringContext.s` and `raw` are Scala 2 macros in scala-library's source, with no bytecode;
// scalac lowers a call of them on a context it cannot fold to `standardInterpolator`.
def interpolateS(context: StringContext, args: Seq[Any]): String =
  StringContext.standardInterpolator(StringContext.processEscapes, args, context.parts)
def interpolateRaw(context: StringContext, args: Seq[Any]): String =
  StringContext.standardInterpolator(part => part, args, context.parts)
