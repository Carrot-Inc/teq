package folded

/** Conditions the typer folds by running the std's methods on the constant arguments, where each call is typed. */
object Marks:
  transparent inline def shout(inline s: String): String = if s.toUpperCase.contains("B") then "has-b" else "no-b"
  transparent inline def pieces(inline s: String): Int = if s.split(",").length == 2 then 2 else -1
