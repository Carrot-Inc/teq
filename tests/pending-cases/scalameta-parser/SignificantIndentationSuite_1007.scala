// Program 1007 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 919, "for-in-
// parens"): scalac 3.8.4's parser accepts it, teq's reports "`yield` or `do` expected". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 95
//     Defn.Object 0 94
//       Term.Name 7 12
//       Template 13 94
//         Template.Body 13 94
//           Defn.Def 15 91
//             Term.Name 19 22
//             Term.Select 27 91
//               Term.Select 27 86
//                 Term.ForYield 28 79
//                   Term.EnumeratorsBlock 32 64
//                     Enumerator.Generator 32 44
//                       Pat.Var 32 33
//                         Term.Name 32 33
//                       Term.Apply 37 44
//                         Term.Name 37 41
//                         Term.ArgClause 41 44
//                           Lit.Int 42 43
//                     Enumerator.Generator 52 64
//                       Pat.Var 52 53
//                         Term.Name 52 53
//                       Term.Apply 57 64
//                         Term.Name 57 61
//                         Term.ArgClause 61 64
//                           Lit.Int 62 63
//                   Term.ApplyInfix 74 79
//                     Term.Name 74 75
//                     Term.Name 76 77
//                     Type.ArgClause 78 78
//                     Term.ArgClause 78 79
//                       Term.Name 78 79
//                 Term.Name 81 86
//               Term.Name 87 91
object W1007 {
def foo =
  (for a <- List(1)
       b <- List(2)
   yield a + b).toSet.size

}
