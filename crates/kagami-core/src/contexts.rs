use crate::{Context, Error, Result};
use rusqlite::{params, Connection};
use uuid::Uuid;

pub(crate) fn all(db: &Connection) -> Result<Vec<Context>> {
    Ok(db.prepare("SELECT id,name,parent_id,color,tracking_locked,position FROM contexts ORDER BY position,id")?
        .query_map([], |r| Ok(Context { id:r.get(0)?, name:r.get(1)?, parent_id:r.get(2)?, color:r.get(3)?, tracking_locked:r.get(4)?, position:r.get(5)? }))?
        .collect::<std::result::Result<_,_>>()?)
}
pub(crate) fn get(db: &Connection, id: &str) -> Result<Context> {
    all(db)?
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(Error::stale)
}
pub(crate) fn check_target(db: &Connection, id: Option<&str>, trackable: bool) -> Result<()> {
    if let Some(id) = id {
        let context = get(db, id)?;
        if trackable && context.tracking_locked {
            return Err(Error::invalid(
                "Tracking is locked for this context. Choose another context.",
            ));
        }
    }
    Ok(())
}
pub(crate) fn name_parts(value: &str) -> Result<(String, String)> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 120 || trimmed.chars().any(char::is_control)
    {
        return Err(Error::invalid(
            "Context names need 1–120 characters and no control characters.",
        ));
    }
    let key = trimmed.to_lowercase();
    if key == "unassigned" {
        return Err(Error::invalid("Unassigned is a reserved name."));
    }
    Ok((trimmed.into(), key))
}
fn check_name(
    db: &Connection,
    name: &str,
    key: &str,
    parent: Option<&str>,
    except: Option<&str>,
) -> Result<()> {
    let taken = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM contexts WHERE parent_id IS ?1 AND name_key=?2 AND id IS NOT ?3)",
        params![parent, key, except],
        |r| r.get::<_, bool>(0),
    )?;
    if taken {
        return Err(Error::invalid(match parent {
            Some(id) => format!("{} already has a context named {name}.", get(db, id)?.name),
            None => format!("A context named {name} already exists at the top level."),
        }));
    }
    Ok(())
}
pub(crate) fn color(value: &str) -> Result<String> {
    let value = value.trim();
    let value = value.strip_prefix('#').unwrap_or(value);
    if ![3, 6].contains(&value.len()) || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::invalid("Use a three- or six-digit HEX color."));
    }
    let hex = if value.len() == 3 {
        value.chars().flat_map(|c| [c, c]).collect()
    } else {
        value.to_owned()
    };
    Ok(format!("#{}", hex.to_uppercase()))
}
fn reorder(db: &Connection, parent: Option<&str>, ids: &[String]) -> Result<()> {
    for (position, id) in ids.iter().enumerate() {
        db.execute(
            "UPDATE contexts SET parent_id=?1,position=?2 WHERE id=?3",
            params![parent, position as i64, id],
        )?;
    }
    Ok(())
}
fn siblings(db: &Connection, parent: Option<&str>, except: &str) -> Result<Vec<String>> {
    Ok(all(db)?
        .into_iter()
        .filter(|c| c.parent_id.as_deref() == parent && c.id != except)
        .map(|c| c.id)
        .collect())
}
pub(crate) fn create(
    db: &Connection,
    value: &str,
    parent: Option<&str>,
    hex: &str,
) -> Result<String> {
    check_target(db, parent, false)?;
    let (name, key) = name_parts(value)?;
    check_name(db, &name, &key, parent, None)?;
    let color = color(hex)?;
    let id = Uuid::new_v4().to_string();
    db.execute(
        "INSERT INTO contexts(id,name,name_key,parent_id,color,position) VALUES(?1,?2,?3,?4,?5,0)",
        params![id, name, key, parent, color],
    )?;
    let mut ids = siblings(db, parent, &id)?;
    ids.insert(0, id.clone());
    reorder(db, parent, &ids)?;
    Ok(id)
}
pub(crate) fn rename(db: &Connection, id: &str, value: &str) -> Result<()> {
    let context = get(db, id)?;
    let (name, key) = name_parts(value)?;
    check_name(db, &name, &key, context.parent_id.as_deref(), Some(id))?;
    db.execute(
        "UPDATE contexts SET name=?1,name_key=?2 WHERE id=?3",
        params![name, key, id],
    )?;
    Ok(())
}
pub(crate) fn set_color(db: &Connection, id: &str, value: &str) -> Result<()> {
    get(db, id)?;
    db.execute(
        "UPDATE contexts SET color=?1 WHERE id=?2",
        params![color(value)?, id],
    )?;
    Ok(())
}
pub(crate) fn move_context(
    db: &Connection,
    id: &str,
    parent: Option<&str>,
    position: usize,
) -> Result<()> {
    let old = get(db, id)?;
    check_target(db, parent, false)?;
    let contexts = all(db)?;
    let mut ancestor = parent;
    while let Some(current) = ancestor {
        if current == id {
            return Err(Error::invalid(
                "A context cannot be moved into its own subtree.",
            ));
        }
        ancestor = contexts
            .iter()
            .find(|c| c.id == current)
            .and_then(|c| c.parent_id.as_deref());
    }
    check_name(db, &old.name, &old.name.to_lowercase(), parent, Some(id))?;
    let mut ids = siblings(db, parent, id)?;
    if position > ids.len() {
        return Err(Error::invalid(
            "The drop position changed. Try moving the context again.",
        ));
    }
    ids.insert(position, id.to_owned());
    reorder(db, parent, &ids)?;
    if old.parent_id.as_deref() != parent {
        reorder(
            db,
            old.parent_id.as_deref(),
            &siblings(db, old.parent_id.as_deref(), id)?,
        )?;
    }
    Ok(())
}
pub(crate) fn set_locked(db: &Connection, id: &str, locked: bool) -> Result<()> {
    get(db, id)?;
    if locked
        && db.query_row(
            "SELECT EXISTS(SELECT 1 FROM recordings WHERE context_id=?1 AND ended_at IS NULL)",
            [id],
            |r| r.get::<_, bool>(0),
        )?
    {
        return Err(Error::invalid(
            "Stop the timer assigned to this context before locking tracking.",
        ));
    }
    db.execute(
        "UPDATE contexts SET tracking_locked=?1 WHERE id=?2",
        params![locked, id],
    )?;
    Ok(())
}
pub(crate) fn delete(db: &Connection, id: &str) -> Result<()> {
    let old = get(db, id)?;
    delete_tree(db, Some(id))?;
    reorder(
        db,
        old.parent_id.as_deref(),
        &siblings(db, old.parent_id.as_deref(), id)?,
    )?;
    Ok(())
}

pub(crate) fn delete_tree(db: &Connection, root: Option<&str>) -> Result<()> {
    // Only recording cascades are needed when each context is deleted after its children.
    let mut tree = db.prepare(
        "WITH RECURSIVE tree(id, depth) AS (
            SELECT id, 0 FROM contexts WHERE id = ?1 OR (?1 IS NULL AND parent_id IS NULL)
            UNION ALL
            SELECT contexts.id, tree.depth + 1 FROM contexts JOIN tree ON contexts.parent_id = tree.id
         )
         SELECT id FROM tree ORDER BY depth DESC",
    )?;
    let ids = tree
        .query_map([root], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut delete = db.prepare("DELETE FROM contexts WHERE id=?1")?;
    for id in ids {
        delete.execute([id])?;
    }
    Ok(())
}
