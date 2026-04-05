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
 * Main example demonstrating memoir (knowledge graph) operations
 */
async function main(): Promise<void> {
  printSection('Alejandria Memoir (Knowledge Graph) Demo (TypeScript)');

  const client = new AlejandriaClient();

  try {
    // Connect to server
    await client.connect();
    console.log('✓ Connected to Alejandria MCP server\n');

    // 1. Create a memoir
    printSection('1. Creating Memoir');

    const memoirName = `typescript-patterns-${Date.now()}`;
    const memoirDesc = 'Knowledge graph of TypeScript design patterns and best practices';

    console.log(`Creating memoir: ${memoirName}`);
    console.log(`Description: ${memoirDesc}\n`);

    const memoir = await client.memoirCreate({
      name: memoirName,
      description: memoirDesc,
    });

    console.log(`✓ Created memoir: ${memoir.name} (ID: ${memoir.id})\n`);

    // 2. Add concepts to the memoir
    printSection('2. Adding Concepts');

    const concepts = [
      {
        name: 'Type Guards',
        description: 'Runtime checks that narrow types in TypeScript, enabling safe type assertions',
      },
      {
        name: 'Generics',
        description: 'Parameterized types that enable writing reusable, type-safe code',
      },
      {
        name: 'Decorators',
        description: 'Special declarations that can be attached to classes, methods, or properties',
      },
      {
        name: 'Union Types',
        description: 'Types formed from two or more types, representing values that may be any of those types',
      },
      {
        name: 'Interface Segregation',
        description: 'SOLID principle: clients should not depend on interfaces they do not use',
      },
    ];

    const conceptIds: Record<string, string> = {};

    // Add concepts in parallel using Promise.all
    console.log('Adding concepts in parallel...\n');
    
    const addConceptPromises = concepts.map(async (concept) => {
      console.log(`Adding: ${concept.name}`);
      console.log(`  Description: ${concept.description.substring(0, 60)}...`);

      const conceptObj = await client.memoirAddConcept({
        memoir: memoir.name,  // Use memoir name, not ID
        name: concept.name,
        definition: concept.description,
      });

      const conceptId = conceptObj.id;
      conceptIds[concept.name] = conceptId;
      console.log(`  ✓ Added with ID: ${conceptId}\n`);
    });

    await Promise.all(addConceptPromises);

    // 3. Link concepts with typed relationships
    printSection('3. Creating Relationships');

    const relationships = [
      {
        from: 'Generics',
        to: 'Type Guards',
        type: 'related_to',
        description: 'Generics and type guards work together for type-safe generic functions',
      },
      {
        from: 'Union Types',
        to: 'Type Guards',
        type: 'prerequisite_of',
        description: 'Union types often require type guards to narrow the type',
      },
      {
        from: 'Decorators',
        to: 'Interface Segregation',
        type: 'example_of',
        description: 'Decorators can help implement interface segregation principle',
      },
      {
        from: 'Generics',
        to: 'Interface Segregation',
        type: 'related_to',
        description: 'Generic constraints enable interface segregation',
      },
    ];

    // Link concepts sequentially to ensure dependencies are met
    for (const rel of relationships) {
      console.log(`Linking: ${rel.from} --[${rel.type}]--> ${rel.to}`);
      console.log(`  Context: ${rel.description}\n`);

      await client.memoirLink({
        memoir: memoir.name,
        source: rel.from,
        target: rel.to,
        relation: rel.type,
      });

      console.log('  ✓ Created link successfully\n');
    }

    // 4. Summary
    printSection('Knowledge Graph Summary');

    console.log(`Memoir: ${memoir.name}`);
    console.log(`ID: ${memoir.id}\n`);
    console.log(`Concepts added: ${concepts.length}`);
    console.log(`Relationships created: ${relationships.length}\n`);

    console.log('Graph structure:');
    console.log('  • Type system features: Type Guards, Union Types, Generics');
    console.log('  • Advanced features: Decorators');
    console.log('  • Design principles: Interface Segregation\n');

    console.log('Key relationships:');
    console.log('  • Generics complement Type Guards for type safety');
    console.log('  • Union Types require Type Guards for narrowing');
    console.log('  • Decorators implement Interface Segregation');
    console.log('  • Generics enable Interface Segregation through constraints\n');

    printSection('Demo Complete');
    console.log('Successfully demonstrated:');
    console.log('  ✓ Memoir creation (memoir_create)');
    console.log('  ✓ Parallel concept addition (memoir_add_concept)');
    console.log('  ✓ Sequential relationship linking (memoir_link)');
    console.log('  ✓ TypeScript async/await patterns\n');

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
