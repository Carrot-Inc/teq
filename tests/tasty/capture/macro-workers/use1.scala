object UseMacroWorkers1:
  def use(x: Any): Boolean = MacroWorkers.isString(x)
