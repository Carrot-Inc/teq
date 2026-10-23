package outside

// A source outside the sourceroot: its document is named by its path as given (SourceFile.relativePath's fallback).
object Other:
  val four = inside.In.twice(2)
