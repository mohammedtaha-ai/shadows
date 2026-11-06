-- Map approval opens Roadmap; no product command writes the structure.
-- This removes database bookkeeping only and never touches project files.
DROP TABLE design_structure_write;
