import { AlejandriaClient, MCPToolError, MCPConnectionError } from './client.js';

/**
 * Print a formatted section header
 */
function printSection(title: string): void {
  console.log(`\n${'='.repeat(60)}`);
  console.log(`  ${title}`);
  console.log(`${'='.repeat(60)}\n`);
}

/**
 * Main example demonstrating memory operations
 */
async function main(): Promise<void> {
  printSection('Alejandria Memory Operations Demo (TypeScript)');

  const client = new AlejandriaClient();

  try {
    // Connect to server
    await client.connect();
    console.log('✓ Connected to Alejandria MCP server\n');

    // 1. Store memories
    printSection('1. Storing Memories');

    const memories = [
      {
        content: 'TypeScript is a strongly typed superset of JavaScript that compiles to plain JavaScript.',
        topic: 'typescript-concepts',
        importance: 'high' as const,
        summary: 'TypeScript basics',
      },
      {
        content: 'Async/await provides a cleaner syntax for handling Promises in JavaScript and TypeScript.',
        topic: 'typescript-concepts',
        importance: 'high' as const,
        summary: 'Async/await pattern',
      },
      {
        content: 'Node.js uses an event-driven, non-blocking I/O model that makes it lightweight and efficient.',
        topic: 'nodejs-concepts',
        importance: 'medium' as const,
        summary: 'Node.js architecture',
      },
    ];

    for (const memory of memories) {
      console.log(`Storing: ${memory.summary}`);
      console.log(`  Topic: ${memory.topic}`);
      console.log(`  Importance: ${memory.importance}`);

      const response = await client.memStore(memory);
      console.log(`  ✓ Stored with ID: ${response.id}\n`);
    }

    // 2. List topics
    printSection('2. Listing All Topics');

    const topics = await client.memListTopics();
    console.log(`Found ${topics.length} topics:\n`);

    for (const topic of topics) {
      console.log(`  • ${topic.topic} (${topic.count} memories)`);
    }
    console.log();

    // 3. Recall memories (note: may have FTS5 search issues on server-side)
    printSection('3. Semantic Search - General Query');

    try {
      const query = 'What is TypeScript?';
      console.log(`Query: "${query}"\n`);

      const results = await client.memRecall({ query, limit: 5 });
      console.log(`Found ${results.length} results:\n`);

      for (const result of results) {
        const similarity = result.similarity?.toFixed(3) || 'N/A';
        console.log(`  [${result.id}] ${result.summary || result.content.substring(0, 60)}`);
        console.log(`  Similarity: ${similarity}`);
        console.log();
      }
    } catch (error) {
      if (error instanceof MCPToolError) {
        console.log(`⚠️  Search failed (known server-side FTS5 issue): ${error.message}\n`);
        console.log('Note: This is a server-side bug, not a client implementation issue.\n');
      } else {
        throw error;
      }
    }

    // 4. Topic-specific recall
    printSection('4. Topic-Specific Search');

    try {
      const query = 'TypeScript';
      console.log(`Query: "${query}"`);
      console.log(`Filter: topic = typescript-concepts\n`);

      const results = await client.memRecall({
        query,
        topic: 'typescript-concepts',
        limit: 5,
      });

      console.log(`Found ${results.length} results:\n`);

      for (const result of results) {
        const similarity = result.similarity?.toFixed(3) || 'N/A';
        console.log(`  [${result.id}] ${result.content.substring(0, 80)}...`);
        console.log(`  Similarity: ${similarity}\n`);
      }
    } catch (error) {
      if (error instanceof MCPToolError) {
        console.log(`⚠️  Search failed (known server-side FTS5 issue): ${error.message}\n`);
      } else {
        throw error;
      }
    }

    printSection('Demo Complete');
    console.log('Successfully demonstrated:');
    console.log('  ✓ Memory storage (mem_store)');
    console.log('  ✓ Topic listing (mem_list_topics)');
    console.log('  ⚠️  Memory recall (mem_recall) - server-side FTS5 issue\n');

  } catch (error) {
    if (error instanceof MCPToolError) {
      console.error(`\n❌ Tool Error: ${error.message}`);
      if (error.data) {
        console.error(`   Data: ${JSON.stringify(error.data)}`);
      }
    } else if (error instanceof MCPConnectionError) {
      console.error(`\n❌ Connection Error: ${error.message}`);
    } else {
      console.error(`\n❌ Unexpected Error: ${error}`);
      if (error instanceof Error) {
        console.error(error.stack);
      }
    }
    process.exit(1);
  } finally {
    // Always close the connection
    await client.close();
  }
}

// Run the example
main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});
