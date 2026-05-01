use crate::error::CliResult;
use crate::output;
use crate::store::Store;

pub fn run(store: &Store, document_id: &str) -> CliResult<()> {
    let document = store.get_document(document_id)?;
    let body = store.document_body(&document)?;
    let files = store.related_files(document_id)?;
    let related = store.related_documents(&[document_id.to_string()], true)?;
    output::print_read_document(&mut std::io::stdout(), &document, &body, &files, &related);
    Ok(())
}
