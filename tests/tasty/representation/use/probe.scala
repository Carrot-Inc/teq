// The oracle's downstream for the reflection's probes (README.md, "The reflection's probes"): a call a probe, each its
// record `probe:<name>`, one a line in the form oracle.py's `teq` reads, so that a shape teq's reflection refuses is
// that probe's alone.
import repr.probe.*
@main def observeProbes(): Unit =
  println(observe.Probe.run[Self]("this-type")) // key probe:this-type
  println(observe.Probe.run[Model.o.Inner.type]("module-class")) // key probe:module-class
  println(observe.Probe.run[Subject.type]("method-info")) // key probe:method-info
  println(observe.Probe.run[Subject.type]("poly-info")) // key probe:poly-info
  println(observe.Probe.run[Subject.type]("annotated-val")) // key probe:annotated-val
  println(observe.Probe.run[Subject.type]("overload")) // key probe:overload
  println(observe.Probe.run[Subject.type]("dependent-result")) // key probe:dependent-result
  println(observe.Probe.run[Subject.type]("repeated")) // key probe:repeated
  println(observe.Probe.run[Subject.type]("inline-param")) // key probe:inline-param
  println(observe.Probe.run[Subject.type]("binder-retained")) // key probe:binder-retained
  println(observe.Probe.run[Subject.type]("adapted-bound")) // key probe:adapted-bound
  println(observe.Probe.run[Subject.type]("adapted-bound-accessor")) // key probe:adapted-bound-accessor
  println(observe.Probe.run[Subject.type]("joint")) // key probe:joint
  println(observe.Probe.run[Subject.type]("annotation-tree")) // key probe:annotation-tree
  println(observe.Probe.run[Subject.type]("by-name-nested")) // key probe:by-name-nested
  println(observe.Probe.run[Subject.type]("by-name-annotated")) // key probe:by-name-annotated
