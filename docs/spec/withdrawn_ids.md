# Withdrawn scenario IDs

The acceptance scenarios that left their tool's spec, each line as it was written there, so
that its ID is never reused (`CONTRIBUTING.md` "Testing"). Two kinds of line come here:

- an area whose every scenario was withdrawn, moved whole with its heading;
- a run of consecutive withdrawn IDs in an area that still has live scenarios, which its spec
  now shows as one range line (`TC-BIN-25 to 33  withdrawn: …`), so whoever writes the area's
  next scenario still sees every number taken.

A line is copied here before it leaves its spec.

`just acceptance` reads this file: an ID here is defined and withdrawn, so a test citing it
and a new scenario reusing it both fail, and every ID a range line compacts must be here.

## `spec/export_upgrader.md`, Player settings

```
EU-SET-05  withdrawn: restated as EU-SET-11 (`spec/export_upgrader.md`), his
           cheek_type in ingame_face.toml since step 0.0.1h
EU-SET-06  withdrawn: no save is stripped (step 8.36b), so settings_from_stripped_record is gone
```

## `spec/save_editor.md`, Transplant (the area withdrawn whole)

```
SE-TRN-01  withdrawn: the save-writing transplant is removed (8.9j5c); its successor writes
           the team's export, SE-GUI-14 (`plans/save_editor.md`)
SE-TRN-02  withdrawn: the save-writing transplant is removed (8.9j5c)
SE-TRN-03  withdrawn: the save-writing transplant is removed (8.9j5c)
```

## `spec/save_editor.md`, Stripped save (the area withdrawn whole)

```
SE-STR-01  withdrawn: no save is stripped, and strip-and-seed and check-stripped are gone (step 8.36b)
SE-STR-02  withdrawn: no save is stripped, and strip-and-seed and check-stripped are gone (step 8.36b)
SE-STR-03  withdrawn: no save is stripped, and strip-and-seed and check-stripped are gone (step 8.36b)
SE-STR-04  withdrawn: no save is stripped, and strip-and-seed and check-stripped are gone (step 8.36b)
SE-STR-05  withdrawn: no save is stripped, and strip-and-seed and check-stripped are gone (step 8.36b)
SE-STR-06  withdrawn: no save is stripped, and strip-and-seed and check-stripped are gone (step 8.36b)
SE-STR-07  withdrawn: no save is stripped, and strip-and-seed and check-stripped are gone (step 8.36b)
SE-STR-08  withdrawn: no save is stripped, and strip-and-seed and check-stripped are gone (step 8.36b)
```

## `spec/team_compiler/README.md`, Structure

```
TC-STR-18  withdrawn: restated as TC-SAV-21 (step 8.36c)
TC-STR-19  withdrawn: restated as TC-SAV-21 (step 8.36c)
```

## `spec/team_compiler/README.md`, Models

```
TC-MOD-58  withdrawn: restated as TC-MOD-96 (step 8.36c)
TC-MOD-59  withdrawn: restated as TC-MOD-97 (step 8.36c)
TC-MOD-60  withdrawn: restated as TC-MOD-98 (step 8.36c)
```

## `spec/team_compiler/README.md`, Bins

```
TC-BIN-10  withdrawn: restated as TC-BIN-25 (`spec/team_compiler/README.md`), which also
           gives the compiled player's gloves and appearance rows
TC-BIN-11  withdrawn: restated as TC-BIN-50 (step 8.36c)
TC-BIN-12  withdrawn: restated as TC-BIN-51 (step 8.36c)
```

## `spec/team_compiler/README.md`, Player tables

```
TC-BIN-25  withdrawn: restated as TC-SAV-18 (step 8.36c)
TC-BIN-26  withdrawn: restated as TC-BIN-52 (step 8.36c)
TC-BIN-27  withdrawn: restated as TC-BIN-53 (step 8.36c)
TC-BIN-28  withdrawn: restated as TC-SAV-18 (step 8.36c)
TC-BIN-29  withdrawn: restated as TC-BIN-54 (step 8.36c)
TC-BIN-30  withdrawn: restated as TC-BIN-55 (step 8.36c)
TC-BIN-31  withdrawn: restated as TC-BIN-49 (`spec/team_compiler/README.md`), the
           face and hair keys read from ingame_face.toml since step 0.0.1h
TC-BIN-32  withdrawn: restated as TC-BIN-50 (step 8.36c)
TC-BIN-33  withdrawn: restated as TC-SAV-21 (step 8.36c)
TC-BIN-37  withdrawn: the installed table rows the bins CPK kept went with the player tables
           (step 8.36c)
TC-BIN-38  withdrawn: restated as TC-BIN-48 (`spec/team_compiler/README.md`), the
           facial hair type read from ingame_face.toml since step 0.0.1h
TC-BIN-47  withdrawn: restated as TC-BIN-60 (step 8.36c)
TC-BIN-48  withdrawn: restated as TC-BIN-61 (step 8.36c)
TC-BIN-49  withdrawn: restated as TC-BIN-62 (step 8.36c)
```

## `spec/team_compiler/README.md`, Shared IDs across a Midcup

```
TC-BIN-34  withdrawn: restated as TC-BIN-56 (step 8.36c)
TC-BIN-35  withdrawn: restated as TC-BIN-57 (step 8.36c)
TC-BIN-36  withdrawn: restated as TC-BIN-58 (step 8.36c)
```
