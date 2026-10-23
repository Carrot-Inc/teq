// Program 113 of tests/fix/scalameta, from scalameta 4.17.4's ControlSyntaxSuite (line 1988, "match-chained
// unindented"): scalac 3.8.4's parser reads it as scalameta does, `xs match { case Nil => "empty" case x :: xs1 =>
// "nonempty" } match { case "empty" => 0 case "nonempty" => 1`, and teq's otherwise. The expectation is scalameta's
// tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from
// the program's first line below.
//   Source 0 119
//     Defn.Object 0 118
//       Term.Name 7 11
//       Template 12 118
//         Template.Body 12 118
//           Term.Match 14 115
//             Term.Match 14 70
//               Term.Name 14 16
//               Term.CasesBlock 23 70
//                 Case 23 42
//                   Term.Name 28 31
//                   Lit.String 35 42
//                 Case 43 70
//                   Pat.ExtractInfix 48 56
//                     Pat.Var 48 49
//                       Term.Name 48 49
//                     Term.Name 50 52
//                     Pat.ArgClause 53 56
//                       Pat.Var 53 56
//                         Term.Name 53 56
//                   Lit.String 60 70
//             Term.CasesBlock 77 115
//               Case 77 94
//                 Lit.String 82 89
//                 Lit.Int 93 94
//               Case 95 115
//                 Lit.String 100 110
//                 Lit.Int 114 115
object W113 {
xs match
case Nil => "empty"
case x :: xs1 => "nonempty"
match
case "empty" => 0
case "nonempty" => 1

}
