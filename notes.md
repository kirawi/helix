# Effective (de)serialization of the undo file

## Requirements
- The system must be able to handle multiple clients attempting to utilize the same undofile
	- Clients may not necessarily possess the same state as each other
	- Clients may be reflective of different discontiguous versions of the document (e.g. external changes)
- The system must be able to handle toggling of undofile usage

## Notes
- The undofile state is well-defined and will always contain a tree rooted at the initial empty revision

## Resolution
- Clients must be able to become aware of differences in their state compared to the changes reflected by the undofile.
- It is possible to allow multiple clients at varying states to utilize the undofile by treating each one as a distinct branch.
	- It is likely the initial intention was to ensure that each history is merged with that of the undofile's history to ensure that the state is maintained before pushing
	- *SCRAPPED* However, this is unintuitive behavior. It should follow Vim and require the other clients to forcefully reload the undofile history if they wish to write to it

- View it as a tree of trees, analagous to branching events. Causality is retained. Each subtree represents the difference of the client's history tree against the undofile's tree. It is necessary to store information to identify the associated file for that difference (a hash).
- Additionally, the objective is to treat the undofile as a master. The undofile's contents should be replicated identically to each of the client's should they be loaded. Because we know causality is retained, we could still push the diff to the undofile even without reloading the history.

## Algorithm
- In save_impl:
```
for all clients:
	if write:
		if outdated_undofile:
			warn "Reload file"
		else:
			write_file
			save_undofile w/ backup mechanism
```
- In Document, we store a hash of the file from when the document's last read/updated the undofile. We use this to check if the current client is outdated.
- The client 
