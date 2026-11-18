// A selector that renames what a search finds is not used by the search: scalac's CheckUnused matches a renamed
// selector by the name the tree uses, the symbol's own; the selector that does not rename it is used.
object P { given number: Long = 7L; given Int = 1 }
object UseGiven { import P.{number as n}; def answer: Long = summon[Long] }
object UseGivenPlain { import P.number; def answer: Long = summon[Long] }

// teq: --werror --wunused imports
// expect: unused_imports_renamed_search.scala:4:29: warning: unused import
// absent: unused_imports_renamed_search.scala:5:
