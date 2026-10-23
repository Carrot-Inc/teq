// Program 789 of tests/fix/scalameta, from scalameta 4.17.4's MinorDottySuite (line 433, "capital-var-pattern-val"):
// scalac 3.8.4's parser accepts it, teq's reports "expected '=', found '@'". The expectation is scalameta's tree
// under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the
// program's first line below.
//   Source 0 43
//     Defn.Object 0 42
//       Term.Name 7 11
//       Template 12 42
//         Template.Body 12 42
//           Defn.Val 14 39
//             Pat.Bind 18 29
//               Pat.Var 18 25
//                 Term.Name 18 25
//               Pat.Wildcard 28 29
//             Term.Apply 32 39
//               Term.Name 32 37
//               Term.ArgClause 37 39
object W789 {
val Private @ _ = flags()

}
