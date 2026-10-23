// Program 1025 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 1428,
// "indented-apply-braces"): scalac 3.8.4's parser reads it as scalameta does, `def method: String = fun(1, 2, 3)( {
// 4 } )`, and teq's otherwise. The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree
// answers to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 65
//     Defn.Object 0 64
//       Term.Name 7 12
//       Template 13 64
//         Template.Body 13 64
//           Defn.Def 15 60
//             Term.Name 19 25
//             Type.Name 27 33
//             Term.Apply 39 60
//               Term.Apply 39 51
//                 Term.Name 39 42
//                 Term.ArgClause 42 51
//                   Lit.Int 43 44
//                   Lit.Int 46 47
//                   Lit.Int 49 50
//               Term.ArgClause 57 60
//                 Term.Block 57 60
//                   Lit.Int 58 59
object W1025 {
def method: String = 
  fun(1, 2, 3) 
    {4} 

}
