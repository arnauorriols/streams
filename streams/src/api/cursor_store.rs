// Rust
use core::fmt;

// 3rd-party
use hashbrown::HashMap;

// IOTA

// Streams
use lets::{address::MsgId, id::Identifier, message::Topic};

// Local

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct CursorStore(HashMap<Topic, InnerCursorStore>);

impl CursorStore {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub(crate) fn new_branch(&mut self, topic: Topic) -> &mut InnerCursorStore {
        self.0.entry(topic).insert(Default::default()).into_mut()
    }

    pub(crate) fn branch(&self, topic: &Topic) -> Option<&InnerCursorStore> {
        self.0.get(topic)
    }

    pub(crate) fn branch_mut(&mut self, topic: &Topic) -> Option<&mut InnerCursorStore> {
        self.0.get_mut(topic)
    }

    pub(crate) fn topics(&self) -> impl Iterator<Item = &Topic> + ExactSizeIterator {
        self.0.keys()
    }

    pub(crate) fn remove(&mut self, id: &Identifier) -> bool {
        let removals = self.0.values_mut().flat_map(|branch| branch.cursors.remove(id));
        removals.count() > 0
    }

    pub(crate) fn get_cursor(&self, topic: &Topic, id: &Identifier) -> Option<usize> {
        self.0.get(topic).and_then(|branch| branch.cursors.get(id).copied())
    }

    pub(crate) fn cursors(&self) -> impl Iterator<Item = (&Topic, &Identifier, usize)> + Clone + '_ {
        self.0
            .iter()
            .flat_map(|(topic, branch)| branch.cursors.iter().map(move |(id, cursor)| (topic, id, *cursor)))
    }

    // TODO: CHANGE RETURN VALUE
    pub(crate) fn get_latest_link(&self, topic: &Topic) -> Option<MsgId> {
        self.0.get(topic).map(|branch| branch.latest_link)
    }
}

#[derive(Clone, PartialEq, Eq, Default)]
pub(crate) struct InnerCursorStore {
    cursors: HashMap<Identifier, usize>,
    latest_link: MsgId,
}

impl InnerCursorStore {
    pub(crate) fn latest_link(&self) -> &MsgId {
        &self.latest_link
    }

    pub(crate) fn set_latest_link(&mut self, latest_link: MsgId) {
        self.latest_link = latest_link;
    }

    pub(crate) fn cursor(&self, identifier: &Identifier) -> Option<usize> {
        self.cursors.get(identifier).copied()
    }

    // USE HANDLER PATTERN TO ENSURE CURSOR AND LATEST_LINK ARE UPDATED
    pub(crate) fn set_cursor(&mut self, id: Identifier, cursor: usize) {
        self.cursors.insert(id, cursor);
    }

    pub(crate) fn contains_cursor(&self, id: &Identifier) -> bool {
        self.cursors.contains_key(id)
    }
}

impl fmt::Debug for InnerCursorStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\t* latest link: {}", self.latest_link)?;
        writeln!(f, "\t* cursors:")?;
        for (id, cursor) in self.cursors.iter() {
            writeln!(f, "\t\t{:?} => {}", id, cursor)?;
        }
        Ok(())
    }
}

impl fmt::Debug for CursorStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "* branches:")?;
        for (topic, branch) in &self.0 {
            writeln!(f, "{:?} => \n{:?}", topic, branch)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::CursorStore;
    use alloc::string::ToString;
    use lets::{
        id::{Ed25519, Identity},
        message::Topic,
    };

    #[test]
    fn branch_store_can_remove_a_cursor_from_all_branches_at_once() {
        let mut branch_store = CursorStore::new();
        let identifier = Identity::Ed25519(Ed25519::from_seed("identifier 1")).to_identifier();
        let topic_1 = Topic::new("topic 1".to_string());
        let topic_2 = Topic::new("topic 2".to_string());

        branch_store.new_branch(topic_1.clone());
        branch_store.new_branch(topic_2.clone());

        branch_store
            .branch_mut(&topic_1)
            .unwrap()
            .set_cursor(identifier.clone(), 10);
        branch_store
            .branch_mut(&topic_2)
            .unwrap()
            .set_cursor(identifier.clone(), 20);

        branch_store.remove(&identifier);

        assert!(!branch_store.is_cursor_tracked(&topic_1, &identifier));
        assert!(!branch_store.is_cursor_tracked(&topic_2, &identifier));
    }
}
